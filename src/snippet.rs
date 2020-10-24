extern crate regex;
use regex::Regex;

use super::file::Reader;
use std::collections::HashMap;
use std::collections::HashSet;

///// Type

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Snippet {
    prefix: String,
    body: String,
    description: String,
}

impl Snippet {
    fn new() -> Self {
        return Snippet {
            prefix: String::new(),
            body: String::new(),
            description: String::new(),
        };
    }
}

struct BandledSnippet {
    meta: SnippetMetaData,
    code: String,
}

type SnippetNames = Vec<String>;
pub type KeyList = HashMap<String, SnippetNames>; // path, names
type SnippetMetaData = HashMap<String, Snippet>; // name, Snippet

#[derive(Debug, PartialEq)]
pub struct Output {
    pub json: String,
    pub name_list: KeyList,
}

#[derive(PartialEq)]
enum SearchStep {
    StartTag,
    Meta,
    EndTag,
    None,
}

#[derive(Debug, PartialEq)]
enum TrimError {
    InvalidName,
    InvalidPrefix,
    InvalidDescription,
    InvalidMeta,
}

///// Tag

const START_TAG: &str = "#PORT#";
const END_TAG: &str = "#PORT_END#";

const NAME_RE: &str = "name:\\s*\"((?:[^\"]|\\.)*)\"";
const PREFIX_RE: &str = "prefix:\\s*\"((?:[^\"]|\\.)*)\"";
const DESC_RE: &str = "description:\\s*\"((?:[^\"]|\\.)*)\"";

const GEN_START_TAG: &str = "[[Generated By PortSnippet]]";
const GEN_END_TAG: &str = "[[PortSnippet End]]";

fn format_tag(tag: &str) -> String {
    return format!("////////// {} (DON'T REMOVE) //////////", tag);
}

fn add_tag(code: &String) -> String {
    let start_tag = format_tag(GEN_START_TAG);
    let end_tag = format_tag(GEN_END_TAG);

    let mut new_code = String::new();
    new_code.push_str(format!("{}\n", start_tag).as_str());
    new_code.push_str(&code);
    new_code.push_str(format!("\n{}\n", end_tag).as_str());

    return new_code;
}

//// Main

// ファイルからスニペットを切り出して、スニペットのjsonを作成する
pub fn make<R: Reader>(
    snippet_reader: R,
    snippet_json_reader: R,
    list_file_reader: &mut R,
    code_filepath_string: String,
) -> Option<Output> {
    // スニペットを切り出す
    let snippet = gen_snippet_json(snippet_reader);
    if snippet.is_none() {
        return None;
    }

    let snippet = snippet.unwrap();

    // 現存してるスニペット情報を取得する + コードの削除をチェック
    let mut all_name_list = get_snippet_namelist(list_file_reader);
    let name_list = filter_namelist(&all_name_list, &code_filepath_string);

    // 現在編集しているファイルに関してリストを持ってくる
    let mut deleted_name_list: Option<HashSet<String>> = None;
    if let Some(name_list) = name_list {
        deleted_name_list = Some(get_deleted_list(&snippet, &name_list));
    }

    // タグを探索し、過去に配置したコードを書き換える
    if let Some(alljson) = gen_alljson(snippet_json_reader, &deleted_name_list, &snippet) {
        // namelist を書き換える
        all_name_list = update_name_list(
            &code_filepath_string,
            &all_name_list,
            &name_list,
            deleted_name_list,
            &snippet,
        );

        return Some(Output {
            json: alljson,
            name_list: all_name_list,
        });
    }

    return None;
}

///// namelist

// lang_identifierごとのnamelistのファイルパスを返す
pub fn get_namelist_filepath(lang_identifier: &str, snippets_dir: &str) -> std::path::PathBuf {
    let mut meta_dir = std::path::PathBuf::from(snippets_dir);
    meta_dir.push(".port_snippet");
    match std::fs::create_dir(&meta_dir) {
        // フォルダを作成
        Err(_) => {}
        Ok(_) => {}
    }

    let mut list_filepath = meta_dir;
    list_filepath.push(format!("{}.json", lang_identifier));
    return list_filepath;
}

fn get_snippet_namelist<T: Reader>(list_file_reader: &mut T) -> KeyList {
    if let Ok(name_list_vec) = serde_json::from_str::<KeyList>(list_file_reader.all().as_str()) {
        return name_list_vec;
    }
    return KeyList::new();
}

// 現存するスニペット情報を返す
// snippets_dir/.port_snippet/hogehoge.jsonを参照する
fn filter_namelist<'a>(
    name_list: &'a KeyList,
    code_filepath_string: &String,
) -> Option<&'a SnippetNames> {
    if name_list.contains_key(code_filepath_string) {
        return Some(&name_list[code_filepath_string]);
    }

    return None;
}

// name_listをupdateする
fn update_name_list(
    code_filepath_string: &String,
    all_name_list: &KeyList,
    name_list: &Option<&SnippetNames>,
    deleted_name_list: Option<HashSet<String>>,
    snippet: &BandledSnippet,
) -> KeyList {
    let mut all_name_list = all_name_list.clone();

    if let Some(name_list) = name_list {
        // すでにnamelistが存在していた場合
        let name_list = name_list.clone(); // all_name_listの参照を指しているので、contains判定のため一度cloneする

        // 新しいスニペット
        for (name, _) in snippet.meta.iter() {
            if !name_list.contains(name) {
                let name_set = all_name_list.get_mut(code_filepath_string);
                name_set.unwrap().push(name.clone());
            }
        }

        // 削除されたスニペット
        if let Some(deleted_name_list) = deleted_name_list {
            for name in name_list.iter() {
                if deleted_name_list.contains(name) {
                    // 削除
                    let name_set = all_name_list.get_mut(code_filepath_string);
                    name_set.unwrap().retain(|x| x.as_str() != name.as_str());
                }
            }
        }
    } else {
        // namelistが存在しない場合
        let mut name_set: SnippetNames = SnippetNames::new();
        for (name, _) in snippet.meta.iter() {
            name_set.push(name.clone());
        }

        all_name_list.insert(code_filepath_string.clone(), name_set);
    }

    return all_name_list;
}

//// Snippet

// 現存してるスニペット情報を取得する + コードの削除をチェック
fn get_deleted_list(snippet: &BandledSnippet, name_list: &SnippetNames) -> HashSet<String> {
    let mut deleted_name_list: HashSet<String> = HashSet::new();

    // 現在編集しているファイルに関してリストを持ってくる
    for existing in name_list.iter() {
        let name = existing.clone();
        if !snippet.meta.contains_key(&name) {
            // スニペットが消えてたら、deleted_name_listにぶちこむ
            deleted_name_list.insert(name);
        }
    }

    return deleted_name_list;
}

// 対象ファイルをトリミングして、スニペット用のjsonの断片を作成
fn gen_snippet_json(reader: impl Reader) -> Option<BandledSnippet> {
    let mut code = String::new();
    let mut is_empty: bool = true;
    let trimmed_map = match trim_code(reader) {
        Ok(t) => t,
        Err(e) => match e {
            TrimError::InvalidName => {
                println!("error: invalid name");
                return None;
            }

            TrimError::InvalidPrefix => {
                println!("error: invalid prefix");
                return None;
            }

            TrimError::InvalidDescription => {
                println!("error: invalid description");
                return None;
            }
            TrimError::InvalidMeta => {
                println!("error: invalid form");
                return None;
            }
        },
    };

    // スニペット用のjsonを生成
    for (name, trimmed) in trimmed_map.iter() {
        let name = name.clone();
        let snippet = trimmed.clone();
        let mut meta = HashMap::new();
        meta.insert(name, snippet);

        if let Ok(json) = serde_json::to_string(&meta) {
            if let Some(json) = json.get(1..json.len() - 1) {
                code.push_str(&json);
                code.push(','); // 常にコンマ打ってOK
                is_empty = false;
            }
        }
    }

    // 何らかの理由でコードが空の場合は空で返す
    if is_empty {
        return Some(BandledSnippet {
            meta: HashMap::new(),
            code: String::from(""),
        });
    }

    return Some(BandledSnippet {
        meta: trimmed_map,
        code: code,
    });
}

// スニペットの全文を作成
fn gen_alljson(
    reader: impl Reader,
    deleted_name_list: &Option<HashSet<String>>,
    bandled: &BandledSnippet,
) -> Option<String> {
    let mut current_step = SearchStep::StartTag;
    let mut allcode = String::new();
    let mut found_tag: bool = false;

    let mut already_ported_json = String::from("{");
    for line in reader.lines() {
        let mut line = line;
        line.push('\n');

        // タグが存在してるなら、jsonを解析する
        // 存在してないならcodeをそのまま貼り付ける
        match current_step {
            SearchStep::StartTag => {
                if line.contains(GEN_START_TAG) {
                    current_step = SearchStep::EndTag;
                    found_tag = true;
                } else {
                    allcode.push_str(&line);
                }
            }
            SearchStep::EndTag => {
                if line.contains(GEN_END_TAG) {
                    // タグが存在する場合
                    current_step = SearchStep::None;
                    let mut found_bracket = false;
                    let mut bracket_index = 0;
                    for c in already_ported_json.as_str().chars().rev() {
                        match c {
                            '}' => {
                                found_bracket = true;
                                break;
                            }
                            _ => {
                                bracket_index += 1;
                            }
                        };
                    }

                    if !found_bracket {
                        return None;
                    }

                    for _ in 0..bracket_index {
                        already_ported_json.pop();
                    }

                    already_ported_json.push_str("}"); // serde_jsonを通すために{}で囲う

                    let mut success = false;
                    if let Ok(already_ported) =
                        serde_json::from_str::<SnippetMetaData>(&already_ported_json)
                    {
                        let mut new_snippets = SnippetMetaData::new();
                        // println!("delete!: {:?}", deleted_name_list);
                        for (name, existing_snippet) in already_ported.iter() {
                            if let Some(deleted_name_list) = deleted_name_list {
                                if deleted_name_list.contains(name) {
                                    // 削除対象は弾く
                                    continue;
                                }
                            }
                            // 詰め替える
                            new_snippets.insert(name.clone(), existing_snippet.clone());
                        }

                        for (name, value) in bandled.meta.iter() {
                            new_snippets.insert(name.clone(), value.clone());
                        }

                        if let Ok(code) = serde_json::to_string(&new_snippets) {
                            let mut code = code
                                .chars()
                                .skip(1)
                                .take(code.len() - 2)
                                .collect::<String>();
                            code.push(',');
                            let formated_code = add_tag(&code);
                            allcode.push_str(&formated_code); // stringに直して書き込む
                            success = true;
                        } else {
                            success = false;
                        }
                    }

                    if !success {
                        println!("failed to perse.");
                        let formated_code = add_tag(&bandled.code);
                        allcode.push_str(&formated_code); // パースできなかったら、そのまま書き込む
                    }
                } else {
                    // タグが存在しない場合
                    already_ported_json.push_str(&line);
                }
            }
            _ => {
                allcode.push_str(&line);
            }
        }
    }

    if found_tag {
        return Some(allcode);
    } else {
        // タグが存在しなかった場合
        let mut bracket_index: usize = 0;
        let mut found_bracket = false;
        for c in allcode.as_str().chars() {
            match c {
                '{' => {
                    found_bracket = true;
                    break;
                }
                _ => {
                    bracket_index += 1;
                }
            };
        }
        bracket_index += 1;
        println!("{}", bracket_index);

        if !found_bracket {
            bracket_index = 0;
        }

        if let (Some(first), Some(second)) = (
            allcode.get(0..bracket_index),
            allcode.get(bracket_index..allcode.len()),
        ) {
            // first: [0,i), second: [i,len)
            let mut allcode = String::from(first.clone());
            let formated_code = add_tag(&bandled.code);

            if !found_bracket {
                allcode.push_str("{");
            }
            allcode.push_str("\n");
            allcode.push_str(&formated_code);
            allcode.push_str(&second);

            if !found_bracket {
                allcode.push_str("\n}");
            }

            println!("{}", allcode);
            return Some(allcode);
        }
    }

    return None;
}

// 対象のコードから、スニペット部分を取り出す
fn trim_code(reader: impl Reader) -> Result<SnippetMetaData, TrimError> {
    let mut current_step = SearchStep::StartTag;
    let mut meta = SnippetMetaData::new();
    let mut code = String::new();
    let mut target = Snippet::new();
    let mut target_name = String::new();

    for line in reader.lines() {
        let mut line = line;
        line.push_str("\n");

        // メタデータを探索
        match &current_step {
            SearchStep::StartTag => {
                if line.contains(START_TAG) {
                    current_step = SearchStep::Meta;
                }
            }
            SearchStep::Meta => {
                if let Some(result) = regex_search(DESC_RE, &line) {
                    // description
                    if result.len() != 2 {
                        return Err(TrimError::InvalidDescription);
                    }

                    target.description = result.get(1).unwrap().to_string(); // 1の方がキャプチャされた文字列
                } else {
                    // すでにnameとprefixが見つかってたなら、EndTagを探すように
                    if !target_name.is_empty() && !target.prefix.is_empty() {
                        current_step = SearchStep::EndTag;
                    }
                }

                if let Some(result) = regex_search(NAME_RE, &line) {
                    // name
                    if result.len() != 2 {
                        return Err(TrimError::InvalidName);
                    }

                    target_name = result.get(1).unwrap().to_string();
                }

                if let Some(result) = regex_search(PREFIX_RE, &line) {
                    // prefix
                    if result.len() != 2 {
                        return Err(TrimError::InvalidPrefix);
                    }

                    target.prefix = result.get(1).unwrap().to_string();
                }
            }
            _ => {}
        }

        // コードを記録
        match &current_step {
            SearchStep::StartTag => {}
            SearchStep::EndTag => {
                if line.contains(END_TAG) {
                    // 詰める
                    target.body = code;
                    meta.insert(target_name.clone(), target);

                    // 諸々初期化
                    code = String::new();
                    current_step = SearchStep::StartTag;
                    target = Snippet::new();
                } else {
                    code.push_str(&line);
                }
            }
            _ => {}
        };
    }

    if current_step != SearchStep::StartTag {
        // タグが正しく閉じてない場合はErr返す
        return Err(TrimError::InvalidMeta);
    }

    // println!("{:?}",meta);
    return Ok(meta);
}

///// Util

fn regex_search(re: &str, text: &String) -> Option<Vec<String>> {
    let re = Regex::new(re).unwrap();
    if let Some(caps) = re.captures(&text) {
        let mut captured: Vec<String> = vec![];
        for i in 0..caps.len() {
            let captured_str = caps.get(i).unwrap().as_str();
            captured.push(String::from(captured_str));
        }

        return Some(captured);
    }

    return None;
}

///// Unit Test

mod tests {
    use crate::snippet::*;

    struct MockReader {
        text: String,
    }

    impl MockReader {
        fn new(text: String) -> MockReader {
            return MockReader { text: text };
        }
    }

    impl Reader for MockReader {
        fn lines(&self) -> Vec<String> {
            return self.text.split('\n').map(|x| String::from(x)).collect();
        }
        fn all(&mut self) -> String {
            return self.text.clone();
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn regexSearch_randomSpacing_valid() {
        let line = String::from("//   name:         \"just_a_mock\"    ");
        let result = regex_search(NAME_RE, &line);

        assert_ne!(result, None);
        let result = result.unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result[1], String::from("just_a_mock"));
    }

    #[test]
    #[allow(non_snake_case)]
    fn trimCode_oneCode_valid() {
        let text = String::from("//#PORT#\n//name:\"just_a_mock\"\n//prefix:\"test_prefix\"\n//description:\"test_desc\"\nfn test() {} \n//#PORT_END#");
        let reader = MockReader::new(text);

        if let Ok(result) = trim_code(reader) {
            assert_eq!(result.len(), 1);
            assert_eq!(result.contains_key("just_a_mock"), true);
            let snippet = &result["just_a_mock"];

            assert_eq!(snippet.prefix, "test_prefix");
            assert_eq!(snippet.description, "test_desc");
            assert_eq!(snippet.body, "fn test() {} \n");
        } else {
            panic!("failed to trim codes");
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn trimCode_someCode_valid() {
        let text = String::from("//#PORT#\n//name:\"just_a_mock\"\n//prefix:\"test_prefix\"\n//description:\"test_desc\"\nfn test() {\nprinln!(\"test!\")\n\nprinln!(\"test2!\")\n} \n//#PORT_END#");
        let reader = MockReader::new(text);

        if let Ok(result) = trim_code(reader) {
            assert_eq!(result.len(), 1);
            assert_eq!(result.contains_key("just_a_mock"), true);
            let snippet = &result["just_a_mock"];

            assert_eq!(snippet.prefix, "test_prefix");
            assert_eq!(snippet.description, "test_desc");
            assert_eq!(
                snippet.body,
                "fn test() {\nprinln!(\"test!\")\n\nprinln!(\"test2!\")\n} \n"
            );
        } else {
            panic!("failed to trim codes");
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn trimCode_asteriskComment_valid() {
        let text = String::from("/*#PORT#*/\n/*name:\"just_a_mock\"*/\n/*prefix:\"test_prefix\"*/\n/*description:\"test_desc\"*/\nfn test() {\nprinln!(\"test!\")\n\nprinln!(\"test2!\")\n} \n/*#PORT_END#*/");
        let reader = MockReader::new(text);

        if let Ok(result) = trim_code(reader) {
            assert_eq!(result.len(), 1);
            assert_eq!(result.contains_key("just_a_mock"), true);
            let snippet = &result["just_a_mock"];

            assert_eq!(snippet.prefix, "test_prefix");
            assert_eq!(snippet.description, "test_desc");
            assert_eq!(
                snippet.body,
                "fn test() {\nprinln!(\"test!\")\n\nprinln!(\"test2!\")\n} \n"
            );
        } else {
            panic!("failed to trim codes");
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn trimCode_sharpComment_valid() {
        let text = String::from("##PORT##\n#name:\"just_a_mock\"#\n#prefix:\"test_prefix\"#\n#description:\"test_desc\"#\nfn test() {\nprinln!(\"test!\")\n\nprinln!(\"test2!\")\n} \n##PORT_END##");
        let reader = MockReader::new(text);

        if let Ok(result) = trim_code(reader) {
            assert_eq!(result.len(), 1);
            assert_eq!(result.contains_key("just_a_mock"), true);
            let snippet = &result["just_a_mock"];

            assert_eq!(snippet.prefix, "test_prefix");
            assert_eq!(snippet.description, "test_desc");
            assert_eq!(
                snippet.body,
                "fn test() {\nprinln!(\"test!\")\n\nprinln!(\"test2!\")\n} \n"
            );
        } else {
            panic!("failed to trim codes");
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn trimCode_withoutName_invalid() {
        let text = String::from("//#PORT#\n//prefix:\"test_prefix\"\n//description:\"test_desc\"\nfn test() {} \n//#PORT_END#");
        let reader = MockReader::new(text);
        let result = trim_code(reader);
        match result {
            Ok(_) => panic!("failed"),
            Err(e) => {
                assert_eq!(e, TrimError::InvalidMeta);
            }
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn trimCode_withoutNameAndPrefix_invalid() {
        let text =
            String::from("//#PORT#\n//description:\"test_desc\"\nfn test() {} \n//#PORT_END#");
        let reader = MockReader::new(text);
        let result = trim_code(reader);
        match result {
            Ok(_) => panic!("failed"),
            Err(e) => {
                assert_eq!(e, TrimError::InvalidMeta);
            }
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn trimCode_withoutPrefix_invalid() {
        let text = String::from("//#PORT#\n//name:\"just_a_mock\"\n//description:\"test_desc\"\nfn test() {} \n//#PORT_END#");
        let reader = MockReader::new(text);
        let result = trim_code(reader);
        match result {
            Ok(_) => panic!("failed"),
            Err(e) => {
                assert_eq!(e, TrimError::InvalidMeta);
            }
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn trimCode_WithoutEndTag_invalid() {
        let text = String::from("//#PORT#\n//name:\"just_a_mock\"\n//prefix:\"test_prefix\"\n//description:\"test_desc\"\nfn test() {} \n");
        let reader = MockReader::new(text);
        let result = trim_code(reader);
        match result {
            Ok(_) => panic!("failed"),
            Err(e) => {
                assert_eq!(e, TrimError::InvalidMeta);
            }
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn addCode_fromScratch_valid() {
        let snippet_text = String::from("//#PORT#\n//name:\"just_a_mock\"\n//prefix:\"test_prefix\"\n//description:\"test_desc\"\nfn test() {\nprinln!(\"test!\")\n\nprinln!(\"test2!\")\n} \n//#PORT_END#");
        let snippet_reader = MockReader::new(snippet_text);

        let namelist_text = String::from("{\"MOCK_PATH\":[\"just_a_mock\"]}");
        let mut namelist_reader = MockReader::new(namelist_text);

        let mock_filename = String::from("MOCK_PATH");
        let snippet_json = String::from("");
        let snippet_json_reader = MockReader::new(snippet_json);

        let result = make(
            snippet_reader,
            snippet_json_reader,
            &mut namelist_reader,
            mock_filename,
        );
        assert_ne!(result, None);

        let result = result.unwrap();
        let expected_json = String::from("{\n////////// [[Generated By PortSnippet]] (DON\'T REMOVE) //////////\n\"just_a_mock\":{\"prefix\":\"test_prefix\",\"body\":\"fn test() {\\nprinln!(\\\"test!\\\")\\n\\nprinln!(\\\"test2!\\\")\\n} \\n\",\"description\":\"test_desc\"},\n////////// [[PortSnippet End]] (DON\'T REMOVE) //////////\n\n\n}");
        assert_eq!(result.json, expected_json);

        assert_eq!(result.name_list.len(), 1);
        assert_eq!(result.name_list.contains_key("MOCK_PATH"), true);
        assert_eq!(result.name_list["MOCK_PATH"].len(), 1);
        assert_eq!(result.name_list["MOCK_PATH"][0], "just_a_mock");
    }

    #[test]
    #[allow(non_snake_case)]
    fn updateName_someCode_valid() {
        let snippet_text = String::from("//#PORT#\n//name:\"modified_name\"\n//prefix:\"test_prefix\"\n//description:\"test_desc\"\nfn test() {\nprinln!(\"test!\")\n\nprinln!(\"test2!\")\n} \n//#PORT_END#");
        let snippet_reader = MockReader::new(snippet_text);

        let namelist_text = String::from("{\"MOCK_PATH\":[\"just_a_mock\"]}");
        let mut namelist_reader = MockReader::new(namelist_text);

        let mock_filename = String::from("MOCK_PATH");
        let snippet_json = String::from("{\n////////// [[Generated By PortSnippet]] (DON\'T REMOVE) //////////\n\"just_a_mock\":{\"prefix\":\"test_prefix\",\"body\":\"fn test() {\\nprinln!(\\\"test!\\\")\\n\\nprinln!(\\\"test2!\\\")\\n} \\n\",\"description\":\"test_desc\"},\n////////// [[PortSnippet End]] (DON\'T REMOVE) //////////\n\n\n}");
        let snippet_json_reader = MockReader::new(snippet_json);

        let result = make(
            snippet_reader,
            snippet_json_reader,
            &mut namelist_reader,
            mock_filename,
        );
        assert_ne!(result, None);

        let result = result.unwrap();
        let expected_json = String::from("{\n////////// [[Generated By PortSnippet]] (DON\'T REMOVE) //////////\n\"modified_name\":{\"prefix\":\"test_prefix\",\"body\":\"fn test() {\\nprinln!(\\\"test!\\\")\\n\\nprinln!(\\\"test2!\\\")\\n} \\n\",\"description\":\"test_desc\"},\n////////// [[PortSnippet End]] (DON\'T REMOVE) //////////\n\n\n}\n");
        assert_eq!(result.json, expected_json);

        assert_eq!(result.name_list.len(), 1);
        assert_eq!(result.name_list.contains_key("MOCK_PATH"), true);
        assert_eq!(result.name_list["MOCK_PATH"].len(), 1);
        assert_eq!(result.name_list["MOCK_PATH"][0], "modified_name");
    }

    #[test]
    #[allow(non_snake_case)]
    fn updatePrefix_someCode_valid() {
        let snippet_text = String::from("//#PORT#\n//name:\"just_a_mock\"\n//prefix:\"modified_prefix\"\n//description:\"test_desc\"\nfn test() {\nprinln!(\"test!\")\n\nprinln!(\"test2!\")\n} \n//#PORT_END#");
        let snippet_reader = MockReader::new(snippet_text);

        let namelist_text = String::from("{\"MOCK_PATH\":[\"just_a_mock\"]}");
        let mut namelist_reader = MockReader::new(namelist_text);

        let mock_filename = String::from("MOCK_PATH");
        let snippet_json = String::from("{\n////////// [[Generated By PortSnippet]] (DON\'T REMOVE) //////////\n\"just_a_mock\":{\"prefix\":\"test_prefix\",\"body\":\"fn test() {\\nprinln!(\\\"test!\\\")\\n\\nprinln!(\\\"test2!\\\")\\n} \\n\",\"description\":\"test_desc\"},\n////////// [[PortSnippet End]] (DON\'T REMOVE) //////////\n\n\n}");
        let snippet_json_reader = MockReader::new(snippet_json);

        let result = make(
            snippet_reader,
            snippet_json_reader,
            &mut namelist_reader,
            mock_filename,
        );
        assert_ne!(result, None);

        let result = result.unwrap();
        let expected_json = String::from("{\n////////// [[Generated By PortSnippet]] (DON\'T REMOVE) //////////\n\"just_a_mock\":{\"prefix\":\"modified_prefix\",\"body\":\"fn test() {\\nprinln!(\\\"test!\\\")\\n\\nprinln!(\\\"test2!\\\")\\n} \\n\",\"description\":\"test_desc\"},\n////////// [[PortSnippet End]] (DON\'T REMOVE) //////////\n\n\n}\n");
        assert_eq!(result.json, expected_json);

        assert_eq!(result.name_list.len(), 1);
        assert_eq!(result.name_list.contains_key("MOCK_PATH"), true);
        assert_eq!(result.name_list["MOCK_PATH"].len(), 1);
        assert_eq!(result.name_list["MOCK_PATH"][0], "just_a_mock");
    }

    #[test]
    #[allow(non_snake_case)]
    fn addCode_someCode_valid() {
        let snippet_text1 = String::from("//#PORT#\n//name:\"just_a_mock\"\n//prefix:\"test_prefix\"\n//description:\"test_desc\"\nfn test() {\nprinln!(\"test!\")\n\nprinln!(\"test2!\")\n} \n//#PORT_END#");
        let snippet_text2 = String::from("//#PORT#\n//name:\"mock2\"\n//prefix:\"prefix2\"\n//description:\"desc2\"\nfn second() {\ntest()}\n//#PORT_END#");

        let snippet_reader = MockReader::new(format!("{}\n{}", snippet_text1, snippet_text2));

        let namelist_text = String::from("{\"MOCK_PATH\":[\"just_a_mock\"]}");
        let mut namelist_reader = MockReader::new(namelist_text);

        let mock_filename = String::from("MOCK_PATH");
        let snippet_json = String::from("{\n////////// [[Generated By PortSnippet]] (DON\'T REMOVE) //////////\n\"just_a_mock\":{\"prefix\":\"test_prefix\",\"body\":\"fn test() {\\nprinln!(\\\"test!\\\")\\n\\nprinln!(\\\"test2!\\\")\\n} \\n\",\"description\":\"test_desc\"},\n////////// [[PortSnippet End]] (DON\'T REMOVE) //////////\n\n\n}");
        let snippet_json_reader = MockReader::new(snippet_json);

        let result = make(
            snippet_reader,
            snippet_json_reader,
            &mut namelist_reader,
            mock_filename,
        );
        assert_ne!(result, None);

        let result = result.unwrap();
        let expected_json = String::from("{\n////////// [[Generated By PortSnippet]] (DON\'T REMOVE) //////////\n\"just_a_mock\":{\"prefix\":\"test_prefix\",\"body\":\"fn test() {\\nprinln!(\\\"test!\\\")\\n\\nprinln!(\\\"test2!\\\")\\n} \\n\",\"description\":\"test_desc\"},\"mock2\":{\"prefix\":\"prefix2\",\"body\":\"fn second() {\\ntest()}\\n\",\"description\":\"desc2\"},\n////////// [[PortSnippet End]] (DON\'T REMOVE) //////////\n\n\n}\n");
        assert_eq!(result.json, expected_json);

        assert_eq!(result.name_list.len(), 1);
        assert_eq!(result.name_list.contains_key("MOCK_PATH"), true);
        assert_eq!(result.name_list["MOCK_PATH"].len(), 2);
        assert_eq!(
            result.name_list["MOCK_PATH"].contains(&"just_a_mock".to_string()),
            true
        );
        assert_eq!(
            result.name_list["MOCK_PATH"].contains(&"mock2".to_string()),
            true
        );
    }

    #[test]
    #[allow(non_snake_case)]
    fn deleteCode_someCode_valid() {
        let snippet_text = String::from("//#PORT#\n//name:\"just_a_mock\"\n//prefix:\"test_prefix\"\n//description:\"test_desc\"\nfn test() {\nprinln!(\"test!\")\n\nprinln!(\"test2!\")\n} \n//#PORT_END#");
        let snippet_reader = MockReader::new(snippet_text);

        let namelist_text = String::from("{\"MOCK_PATH\":[\"just_a_mock\",\"mock2\"]}");
        let mut namelist_reader = MockReader::new(namelist_text);

        let mock_filename = String::from("MOCK_PATH");
        let snippet_json = String::from("{\n////////// [[Generated By PortSnippet]] (DON\'T REMOVE) //////////\n\"just_a_mock\":{\"prefix\":\"test_prefix\",\"body\":\"fn test() {\\nprinln!(\\\"test!\\\")\\n\\nprinln!(\\\"test2!\\\")\\n} \\n\",\"description\":\"test_desc\"},\"mock2\":{\"prefix\":\"prefix2\",\"body\":\"fn second() {\\ntest()}\\n\",\"description\":\"desc2\"},\n////////// [[PortSnippet End]] (DON\'T REMOVE) //////////\n\n\n}");
        let snippet_json_reader = MockReader::new(snippet_json);

        let result = make(
            snippet_reader,
            snippet_json_reader,
            &mut namelist_reader,
            mock_filename,
        );
        assert_ne!(result, None);

        let result = result.unwrap();
        let expected_json = String::from("{\n////////// [[Generated By PortSnippet]] (DON\'T REMOVE) //////////\n\"just_a_mock\":{\"prefix\":\"test_prefix\",\"body\":\"fn test() {\\nprinln!(\\\"test!\\\")\\n\\nprinln!(\\\"test2!\\\")\\n} \\n\",\"description\":\"test_desc\"},\n////////// [[PortSnippet End]] (DON\'T REMOVE) //////////\n\n\n}\n");
        assert_eq!(result.json, expected_json);

        assert_eq!(result.name_list.len(), 1);
        assert_eq!(result.name_list.contains_key("MOCK_PATH"), true);
        assert_eq!(result.name_list["MOCK_PATH"].len(), 1);
        assert_eq!(
            result.name_list["MOCK_PATH"].contains(&"just_a_mock".to_string()),
            true
        );
        assert_eq!(
            result.name_list["MOCK_PATH"].contains(&"mock2".to_string()),
            false
        );
    }
}

extern crate regex;
use regex::Regex;

use std::collections::HashMap;
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Snippet {
    prefix: String,
    body: String,
    description: String,
}

impl Snippet {
    fn new()-> Self {
        return Snippet {
            prefix:String::from(""),
            body:String::from(""),
            description:String::from(""),
        }
    }
}

struct BandledSnippet {
    meta: SnippetMetaData,
    code: String,
}

type SnippetNames = Vec<String>;
type KeyList = HashMap<String,SnippetNames>; // path, names

type SnippetMetaData = HashMap<String,Snippet>; // name, Snippet



enum SearchStep {
    StartTag,
    Name,
    Prefix,
    Description,
    EndTag,
    None,
}

enum TrimError {
    InvalidName,
    InvalidPrefix,
    InvalidDescription,
}


const START_TAG: &str = "#PORT#";
const END_TAG: &str = "#PORT_END#";

const NAME_RE: &str = "name: \"((?:[^\"]|\\.)*)\"";
const PREFIX_RE: &str = "prefix: \"((?:[^\"]|\\.)*)\"";
const DESC_RE: &str = "description: \"((?:[^\"]|\\.)*)\"";

const GEN_START_TAG: &str = "[[Generated By PortSnippet]]";
const GEN_END_TAG: &str = "[[PortSnippet End]]";

fn format_tag(tag: &str)-> String {
    return format!("////////// {} (DON'T REMOVE) //////////",tag);
}

fn add_tag(code: &String)-> String {
    let start_tag = format_tag(GEN_START_TAG);
    let end_tag = format_tag(GEN_END_TAG);

    let mut new_code = String::new();
    new_code.push_str(format!("{}\n", start_tag).as_str());
    new_code.push_str(&code);
    new_code.push_str(format!("\n{}\n", end_tag).as_str());

    return new_code;
}

pub fn make(lang_identifier: String, snippets_dir: &str, code_filepath: &std::path::PathBuf) {
    // スニペットを切り出す
    let snippet = gen_snippet_json(code_filepath);
    let code_filepath_string = std::path::PathBuf::from(code_filepath).into_os_string().into_string().clone().unwrap();

    if (snippet.is_none()) {
        return;
    }

    let snippet = snippet.unwrap();
    let mut deleted_name_list: HashSet<String> = HashSet::new();

    // 現存してるスニペット情報を取得する + コードの削除をチェック
    let list_filepath = get_namelist_filepath(lang_identifier.as_str(),snippets_dir);
    let mut all_name_list = get_snippet_namelist(&list_filepath);
    let name_list = filter_namelist(&all_name_list, &code_filepath_string); 

    // 現在編集しているファイルに関してリストを持ってくる
    if let Some(name_list) = name_list {
        for existing in name_list.iter() {
            let name = existing.clone();
            if !snippet.meta.contains_key(&name) { // スニペットが消えてたら、deleted_name_listにぶちこむ
                deleted_name_list.insert(name);
            }
        }
    }   

    let snippet_filename = format!("{}.json", lang_identifier);
    let mut snippet_filepath = std::path::PathBuf::from(snippets_dir);

    snippet_filepath.push(snippet_filename);
    println!("{:?}", snippet_filepath);

    
    let file = match File::open(&snippet_filepath) {
        Ok(f) => f,
        Err(ref error) if error.kind() == std::io::ErrorKind::NotFound => {
            match File::create(&snippet_filepath) {
                Ok(f) => f,
                Err(_) => {
                    panic!("cannot create snippet file!");
                }
            }
        }
        Err(_) => {
            panic!("cannot open snippet file!");
        }
    };

    // タグを探索し、過去に配置したコードを書き換える
    if let Some(allcode) = gen_allcode(file, &deleted_name_list, &snippet) {
        super::file::write_file(&snippet_filepath, allcode);
        
        // TODO: snippets_dir/.port_snippet/hogehoge.json を書き換える

        if let Some(name_list) = name_list { // すでにnamelistが存在していた場合
            let name_list = name_list.clone(); // all_name_listの参照を指しているので、contains判定のため一度cloneする

            // 新しいスニペット
            for (name,_) in snippet.meta.iter() {
                if !name_list.contains(name) { 
                    let name_set = all_name_list.get_mut(&code_filepath_string);
                    name_set.unwrap().push(name.clone());
                }
            }

            // 削除されたスニペット
            for name in name_list.iter() {
                if deleted_name_list.contains(name) {
                    let name_set = all_name_list.get_mut(&code_filepath_string);
                    name_set.unwrap().retain(|x| x.as_str() != name.as_str()); // 削除
                }
            }
        }
        else { // namelistが存在しない場合
            let mut name_set: SnippetNames = SnippetNames::new();
            for (name,_) in snippet.meta.iter() {
                name_set.push(name.clone());
            }

            all_name_list.insert(code_filepath_string,name_set);
        }

        // 新しいnamelistを書き込む
        if let Ok(name_list_string) = serde_json::to_string::<KeyList>(&all_name_list) {
            super::file::write_file(&list_filepath, name_list_string);
        }
    }
}

// lang_identifierごとのnamelistのファイルパスを返す
fn get_namelist_filepath(lang_identifier: &str, snippets_dir: &str)-> std::path::PathBuf {
    let mut meta_dir = std::path::PathBuf::from(snippets_dir);
    
    meta_dir.push(".port_snippet");
    match std::fs::create_dir(&meta_dir) { // フォルダを作成
        Err(_) => {},
        Ok(_) => {},
    }

    let mut list_filepath = meta_dir;
    list_filepath.push(format!("{}.json",lang_identifier));
    return list_filepath;
}


fn get_snippet_namelist(list_filepath: &std::path::PathBuf)-> KeyList {
    if let Ok(name_list_vec) = serde_json::from_str::<KeyList>(super::file::read_file(&list_filepath).as_str()) {
        return name_list_vec;
    }
    
    return KeyList::new();
}

// 現存するスニペット情報を返す
// snippets_dir/.port_snippet/hogehoge.jsonを参照する
fn filter_namelist<'a>(name_list: &'a KeyList, code_filepath_string: &String)-> Option<&'a SnippetNames> {    
    if name_list.contains_key(code_filepath_string) {
        return Some(&name_list[code_filepath_string]);
    }

    return None;
}

// スニペット用のjsonの断片を作成
fn gen_snippet_json(code_filepath: &std::path::PathBuf) -> Option<BandledSnippet> {
    let mut code = String::new();
    let mut is_empty: bool = true;

    let file = match File::open(&code_filepath) {
        Ok(f) => f,
        Err(e) => {
            println!("{:?}", e);
            return None;
        }
    };

    let trimmed_map = match trim_code(file) {
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
        },
    };

    // スニペット用のjsonを生成
    for (name,trimmed) in trimmed_map.iter() {
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
        return Some(BandledSnippet{meta: HashMap::new(),code: String::from("")});
    }

    return Some(BandledSnippet {
        meta: trimmed_map,
        code: code,
    });
}

// スニペットの全文を作成
fn gen_allcode(file: File, deleted_name_list: &HashSet<String>, bandled: &BandledSnippet) -> Option<String> {
    let mut current_step = SearchStep::StartTag;
    let mut allcode = String::new();
    let mut found_tag: bool = false;

    let mut already_ported_code = String::from("{");
    for line in BufReader::new(file).lines() {
        let mut line = line.unwrap();
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
                if line.contains(GEN_END_TAG) { // タグが存在する場合
                    current_step = SearchStep::None;
                    already_ported_code.pop();
                    already_ported_code.pop();
                    already_ported_code.push_str("}");
                    
                    let mut success = false;
                    if let Ok(already_ported) = serde_json::from_str::<SnippetMetaData>(&already_ported_code) {
                        let mut new_snippets = SnippetMetaData::new();
                        println!("delete!: {:?}",deleted_name_list);
                        for (name, existing_snippet) in already_ported.iter() {
                            if deleted_name_list.contains(name) { // 削除対象は弾く
                                continue;
                            }
                            
                            if bandled.meta.contains_key(name) { // 書き換える対象は書き換えて、
                                new_snippets.insert(name.clone(),bandled.meta[name].clone());
                            }
                            else { // 書き換えない対象はそのままにしておく
                                new_snippets.insert(name.clone(),existing_snippet.clone());
                            }
                        }
                        if let Ok(code) = serde_json::to_string(&new_snippets) {
                            let mut code = code.chars().skip(1).take(code.len()-2).collect::<String>();
                            code.push(',');
                            let formated_code = add_tag(&code);
                            allcode.push_str(&formated_code); // stringに直して書き込む
                            success = true;
                        }
                        else {
                            success = false;
                        }
                    }

                    if !success {
                        println!("failed to perse.");
                        let formated_code = add_tag(&bandled.code);
                        allcode.push_str(&formated_code); // パースできなかったら、そのまま書き込む
                    }
                }
                else { // タグが存在しない場合
                    already_ported_code.push_str(&line);
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
fn trim_code(file: File) -> Result<SnippetMetaData, TrimError> {
    let mut current_step = SearchStep::StartTag;
    let mut meta = SnippetMetaData::new();
    let mut code = String::new();
    let mut target = Snippet::new();
    let mut target_name = String::new();

    for line in BufReader::new(file).lines() {
        let mut line = line.unwrap();
        line.push_str("\n");

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

        // メタデータを探索
        match &current_step {
            SearchStep::StartTag => {
                if line.contains(START_TAG) {
                    current_step = SearchStep::Name;
                }
            }
            SearchStep::Name => {
                if let Some(result) = regex_search(NAME_RE, &line) {
                    if result.len() != 2 {
                        return Err(TrimError::InvalidName);
                    }

                    target_name = result.get(1).unwrap().to_string(); // 1の方がキャプチャされた文字列
                    current_step = SearchStep::Prefix;
                }
            }
            SearchStep::Prefix => {
                if let Some(result) = regex_search(PREFIX_RE, &line) {
                    if result.len() != 2 {
                        return Err(TrimError::InvalidPrefix);
                    }

                    target.prefix = result.get(1).unwrap().to_string();
                    current_step = SearchStep::Description;
                }
            }
            SearchStep::Description => {
                if let Some(result) = regex_search(DESC_RE, &line) {
                    if result.len() != 2 {
                        return Err(TrimError::InvalidDescription);
                    }

                    target.description = result.get(1).unwrap().to_string();
                    current_step = SearchStep::EndTag;
                }
            }
            _ => {}
        }
    }

    return Ok(meta);
}

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

extern crate regex;
use regex::Regex;

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};

#[derive(Serialize, Deserialize, Debug)]
struct Snippet {
    prefix: String,
    body: String,
    description: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct SnippetMetaData {
    name: String,
    snippet: Snippet,
}

impl SnippetMetaData {
    fn new() -> SnippetMetaData {
        SnippetMetaData {
            name: String::from("undefined"),
            snippet: Snippet {
                prefix: String::from(""),
                description: String::from(""),
                body: String::from(""),
            },
        }
    }
}

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

const GEN_START_TAG: &str = "/////  Generated By Dynamic Snippet /////";
const ALERT_MESSAGE: &str = "///// DO NOT remove this code ! /////";
const GEN_END_TAG: &str = "//////////  End  //////////";

pub fn make(lang_identifier: String, snippets_dir: &str, code_filepath: &std::path::PathBuf) {
    let mut code = String::new();
    let mut is_empty: bool = true;

    let file = match File::open(&code_filepath) {
        Ok(f) => f,
        Err(e) => {
            println!("{:?}", e);
            return;
        }
    };

    let trimmed_vec = match trim_code(file) {
        Ok(t) => t,
        Err(e) => match e {
            TrimError::InvalidName => {
                println!("error: invalid name");
                return;
            }

            TrimError::InvalidPrefix => {
                println!("error: invalid prefix");
                return;
            }

            TrimError::InvalidDescription => {
                println!("error: invalid description");
                return;
            }
        },
    };

    // スニペット用のjsonを生成
    for trimmed in trimmed_vec {
        let snippet = trimmed.snippet;
        let name = trimmed.name;
        let mut meta = HashMap::new();
        meta.insert(name, snippet);

        if let Ok(json) = serde_json::to_string(&meta) {
            if let Some(json) = json.get(1..json.len() - 1) {
                code.push_str(format!("{}\n{}\n", GEN_START_TAG, ALERT_MESSAGE).as_str());
                code.push_str(&json);
                code.push('\n');
                code.push_str(format!("{}\n", GEN_END_TAG).as_str());
                is_empty = false;
            }
        }
    }

    // 何らかの理由でコードが空の場合は弾く
    if is_empty {
        return;
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

    // タグを探索し、過去にDynamiSnippetが配置したコードを書き換える

    let mut current_step = SearchStep::StartTag;
    let mut allcode = String::new();
    let mut found_tag: bool = false;

    for line in BufReader::new(file).lines() {
        let mut line = line.unwrap();
        line.push('\n');

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
                    current_step = SearchStep::None;
                    allcode.push_str(&code); // snippetを書き込む
                }
            }
            _ => {
                allcode.push_str(&line);
            }
        }
    }

    if found_tag {
        super::file::write_file(&snippet_filepath, allcode);
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

            if !found_bracket {
                allcode.push_str("{");
            }
            allcode.push_str("\n");
            allcode.push_str(&code);
            allcode.push_str(&second);

            if !found_bracket {
                allcode.push_str("\n}");
            }

            println!("{}", allcode);
            super::file::write_file(&snippet_filepath, allcode);
        }
    }

    // println!("{:?}", code);
}

fn trim_code(file: File) -> Result<Vec<SnippetMetaData>, TrimError> {
    let mut metas: Vec<SnippetMetaData> = vec![];

    let mut current_step = SearchStep::StartTag;
    let mut meta = SnippetMetaData::new();
    let mut code = String::new();

    for line in BufReader::new(file).lines() {
        let mut line = line.unwrap();
        line.push_str("\n");

        // コードを記録
        match &current_step {
            SearchStep::StartTag => {}
            SearchStep::EndTag => {
                if line.contains(END_TAG) {
                    metas.push(SnippetMetaData {
                        // 詰め替える
                        name: String::from(&meta.name),
                        snippet: Snippet {
                            prefix: String::from(&meta.snippet.prefix),
                            description: String::from(&meta.snippet.description),
                            body: code,
                        },
                    });
                    code = String::new();
                    meta = SnippetMetaData::new();
                    current_step = SearchStep::StartTag;
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

                    meta.name = result.get(1).unwrap().to_string(); // 1の方がキャプチャされた文字列
                    current_step = SearchStep::Prefix;
                }
            }
            SearchStep::Prefix => {
                if let Some(result) = regex_search(PREFIX_RE, &line) {
                    if result.len() != 2 {
                        return Err(TrimError::InvalidPrefix);
                    }

                    meta.snippet.prefix = result.get(1).unwrap().to_string();
                    current_step = SearchStep::Description;
                }
            }
            SearchStep::Description => {
                if let Some(result) = regex_search(DESC_RE, &line) {
                    if result.len() != 2 {
                        return Err(TrimError::InvalidDescription);
                    }

                    meta.snippet.description = result.get(1).unwrap().to_string();
                    current_step = SearchStep::EndTag;
                }
            }
            _ => {}
        }
    }

    return Ok(metas);
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

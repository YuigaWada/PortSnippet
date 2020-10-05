#[derive(Serialize, Deserialize, Debug)]
struct Languages {
    lang: Vec<Language>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Language {
    name: String,
    identifier: String,
    extension: String,
}

pub fn get_lang(extension: String) -> Option<String> {
    let langs = get_langdata().lang;

    for lang in langs {
        if lang.extension == extension {
            return Some(lang.identifier);
        }
    }

    return None;
}

fn get_langdata() -> Languages {
    let mut lang_json_path = std::env::current_exe().expect("cannot get current_exe");
    lang_json_path.pop();
    lang_json_path.push("lang.json");

    let contents = super::file::read_file(&lang_json_path);
    let langs: Languages = serde_json::from_str(&contents).expect("cannot get lang.json");
    return langs;
}

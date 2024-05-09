#[macro_use]
extern crate dotenv_codegen;

use cursive::{views::{Button, LinearLayout, TextView}, view::Scrollable};
use dotenv::dotenv;
use json::{self, JsonValue};
use reqwest;
use std::{collections::HashMap, env};

#[allow(dead_code)]
struct Rom {
    // -> Different / similar words
    // scharf
    headword: String,
    // scharf [Sarf] ADJ
    headword_full: String,
    // adjective and adverb
    wordclass: String,
    // Meaning
    arabs: Vec<Arab>,
}

#[allow(dead_code)]
struct Arab {
    // -> Meaning
    // 1. scharf (schneidend, stark gewuerzt)
    header: String,
    translations: Vec<Translation>,
}

#[allow(dead_code)]
struct Translation {
    // scharf
    source: String,
    // злой
    target: String,
}

fn strip_html(source: &str) -> String {
    let mut data = String::new();
    let mut inside = false;
    for c in source.chars() {
        if c == '<' {
            inside = true;
            continue;
        }
        if c == '>' {
            inside = false;
            continue;
        }
        if !inside {
            data.push(c);
        }
    }
    return data;
}

#[tokio::main]
async fn main() -> Result<(), reqwest::Error> {
    dotenv().ok();

    let mut args: Vec<String> = env::args().collect::<Vec<String>>();
    args.remove(0);

    if args.len() < 2 {
        panic!(
            "Input language and word not specified!\nTemplate: <input lang> [output lang] <word>"
        );
    }

    // en, de, ru - 2 letters, basically
    let input_lang = args[0].clone();
    let mut output_lang_mentioned: bool = false;
    let output_lang = match args[1].as_str() {
        "en" => {
            output_lang_mentioned = true;
            "en"
        }
        "de" => {
            output_lang_mentioned = true;
            "de"
        }
        "ru" => {
            output_lang_mentioned = true;
            "ru"
        }
        _ => match input_lang.as_str() {
            "en" => "de",
            "ru" => "de",
            "de" => "ru",
            _ => "ru",
        },
    };
    args.remove(0);
    if output_lang_mentioned {
        args.remove(0);
    }
    let to_translate = args.join(" ");

    let request_builder = reqwest::Client::new().get(format!(
        "https://api.pons.com/v1/dictionary?l={}{}&q={}",
        input_lang, output_lang, to_translate
    ));
    let request = request_builder.header(
        "X-Secret",
        dotenv!("PONS_API_KEY"),
    );
    let response = request.send().await?;

    if response.status().is_success() {
        let body = response.text().await?;
        let json_response = json::parse(&body).unwrap();
        let hits = &json_response[0]["hits"];
        let roms = hits
            .members()
            .map(|h| &h["roms"])
            .collect::<Vec<&JsonValue>>()[0];
        // dbg!(&roms[0]["headword"]);
        let arabs = &roms[0]["arabs"];
        // HashMap<the "header" word, Vec<(German (e.g.) word/example, translation)>>
        let mut meanings: HashMap<&str, Vec<(&str, &str)>> = HashMap::new();
        for arab in arabs.members() {
            let header_word = arab["header"].as_str().unwrap();
            let translations: Vec<(&str, &str)> = arab["translations"]
                .members()
                .into_iter()
                .map(|e| (e["source"].as_str().unwrap(), e["target"].as_str().unwrap()))
                .collect();
            meanings.insert(header_word, translations);
        }

        let mut siv = cursive::default();
        let mut contents = LinearLayout::vertical().child(TextView::new(format!(
            "The word \"{}\" has the following meanings:",
            to_translate
        )));
                contents.add_child(TextView::new(" "));
        for (header_word, translations) in meanings.into_iter() {
            contents.add_child(TextView::new(format!("{}", strip_html(header_word))));
            for (original, translation) in translations {
                contents.add_child(TextView::new(format!(
                    "    -> Original: {}",
                    strip_html(original)
                )));
                contents.add_child(TextView::new(format!(
                    "    Translation: {}\n",
                    strip_html(translation)
                )));
                contents.add_child(TextView::new(" "));
            }
        }

        contents.add_child(Button::new("Quit", |s| s.quit()));
        siv.add_layer(contents.scrollable());
        siv.run();

        // println!("{}", body);
    } else {
        println!("Request failed with status code {}", response.status());
    }

    Ok(())
}

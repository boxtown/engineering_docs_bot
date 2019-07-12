use super::utils::sanitize_text;
use failure::{bail, Error};
use rusoto_comprehend::BatchDetectKeyPhrasesRequest as Request;
use rusoto_comprehend::BatchDetectKeyPhrasesResponse as Response;
use rusoto_comprehend::{BatchItemError, Comprehend, ComprehendClient};
use rusoto_core::Region;
use std::collections::HashMap;

pub type Keyword = String;
pub type Keywords = Vec<Keyword>;

pub type DocPath = String;
pub type DocPaths = Vec<DocPath>;

pub type KeywordMap = HashMap<DocPath, Keywords>;
pub type ReverseKeywordMap = HashMap<Keyword, DocPaths>;

pub fn analyze_keywords(text: String) -> Result<Keywords, Error> {
    let client = ComprehendClient::new(Region::UsEast1);
    let chars: Vec<char> = text.chars().collect();

    // AWS Comprehend has a per-text size limit of 5k chars
    let chunks: Vec<String> = chars
        .chunks(2_500)
        .map(|slice| {
            let s: String = slice.iter().collect();
            sanitize_text(&s)
        })
        .take(25)
        .collect();

    let request = Request {
        language_code: "en".to_owned(),
        text_list: chunks,
    };

    // TODO: this can become async by implementing streams but doing so is too complex
    // for hack day
    let response = client.batch_detect_key_phrases(request).sync()?;
    process_analyze_response(response)
}

pub fn reverse_keyword_map(keyword_map: &KeywordMap) -> ReverseKeywordMap {
    let mut result = ReverseKeywordMap::new();
    for (doc_path, keywords) in keyword_map.iter() {
        for keyword in keywords.iter() {
            result
                .entry(keyword.to_owned())
                .or_insert_with(Vec::new)
                .push(doc_path.to_owned());
        }
    }

    result
}

fn process_analyze_response(response: Response) -> Result<Keywords, Error> {
    if !response.error_list.is_empty() {
        bail!(coalesce_errors(&response.error_list));
    }

    let keywords = response
        .result_list
        .into_iter()
        .filter_map(|result| result.key_phrases)
        .flat_map(|key_phrases| {
            key_phrases
                .into_iter()
                .filter(|key_phrase| key_phrase.score.unwrap_or(0.0) >= 0.85)
                .flat_map(|key_phrase| key_phrase.text)
        })
        .collect();

    Ok(keywords)
}

fn coalesce_errors(error_list: &[BatchItemError]) -> String {
    error_list
        .iter()
        .filter_map(|e| e.error_message.to_owned())
        .collect::<Vec<String>>()
        .join("\n")
}

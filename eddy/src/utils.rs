use serde::{Deserialize, Deserializer};

/// Attempts to deserialize `T`, returning the default value for `T` if the deserializer
/// could not be deserialize as type `T`
pub fn deserialize_or_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    T::deserialize(deserializer).or_else(|_| Ok(T::default()))
}

/// Strips the string of any extra whitespace and non-sentence-terminating
/// punctuation (.!?), lowercases all characters
pub fn sanitize_text(text: &str) -> String {
    text.lines()
        .map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            words
                .iter()
                .enumerate()
                .map(|(i, word)| {
                    if i == words.len() - 1 {
                        sanitize_word(word, true)
                    } else {
                        sanitize_word(word, false)
                    }
                })
                .collect::<Vec<String>>()
                .join(" ")
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<String>>()
        .join("\n")
}

fn sanitize_word(word: &str, preserve_end_marks: bool) -> String {
    let len = word.len();
    let end_marks = ".?!";

    word.to_lowercase()
        .char_indices()
        .filter(|(i, c)| {
            let last_char = *i == len - 1;
            let sentence_end_clause = last_char && preserve_end_marks && end_marks.contains(*c);
            c.is_alphanumeric() || sentence_end_clause
        })
        .map(|(_, c)| c)
        .collect()
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sanitize_text() {
        let word = "A.b.C";
        assert_eq!(sanitize_text(&word), "abc");

        let sentence = "Mary had a \"little\" lamb?";
        assert_eq!(sanitize_text(&sentence), "mary had a little lamb?");

        let multiline = "Enter the following command:\n```\necho \"Hello, World!\"\n```";
        assert_eq!(
            sanitize_text(&multiline),
            "enter the following command\necho hello world"
        );

        let with_code_block = r#"
            This is an example of text with a:

            ```
            code block
            ```
        "#;
        assert_eq!(
            sanitize_text(&with_code_block),
            "this is an example of text with a\ncode block"
        );
    }
}

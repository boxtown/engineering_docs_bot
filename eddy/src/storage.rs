pub mod redis {
    use super::super::analysis::ReverseKeywordMap;
    use failure::Error;

    pub fn save_reverse_keyword_map(
        conn_str: &str,
        reverse_keyword_map: ReverseKeywordMap,
    ) -> Result<(), Error> {
        let client = redis::Client::open(conn_str)?;
        let conn = client.get_connection()?;

        reverse_keyword_map
            .iter()
            .fold(redis::pipe().atomic(), |pipe, (keyword, doc_paths)| {
                doc_paths.iter().fold(pipe, |pipe, doc_path| {
                    pipe.cmd("RPUSH").arg(keyword).arg(doc_path)
                })
            })
            .query(&conn)?;

        Ok(())
    }
}
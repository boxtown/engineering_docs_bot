use data_encoding::HEXUPPER;
use eddy::utils::deserialize_or_default;
use failure::{bail, format_err, Error};
use lambda_http::{http, Body};
use ring::{digest, hmac};
use serde::Deserialize;
use std::str;

#[derive(Eq, PartialEq, Debug, Clone, Copy, Deserialize)]
pub enum EventType {
    Skip,

    #[serde(rename = "app.mention")]
    AppMention,
}

impl Default for EventType {
    fn default() -> Self {
        EventType::Skip
    }
}

#[derive(Eq, PartialEq, Debug, Clone, Copy, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventWrapperType {
    EventCallback,
    UrlVerification,
}

impl Default for EventWrapperType {
    fn default() -> Self {
        EventWrapperType::EventCallback
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Event {
    #[serde(deserialize_with = "deserialize_or_default")]
    pub r#type: EventType,

    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Deserialize)]
pub struct EventWrapper {
    #[serde(deserialize_with = "deserialize_or_default")]
    pub r#type: EventWrapperType,

    #[serde(default)]
    pub event: Event,

    #[serde(default)]
    pub challenge: String,
}

pub fn authenticate_request(
    parts: &http::request::Parts,
    body: &Body,
    signing_secret: &str,
) -> Result<String, Error> {
    let timestamp = parts
        .headers
        .get("X-Slack-Request-Timestamp")
        .ok_or_else(|| format_err!("Missing timestamp header"))
        .map(|v| v.as_bytes())?;

    let expected_signature = parts
        .headers
        .get("X-Slack-Signature")
        .ok_or_else(|| format_err!("Missing expected signature header"))
        .map(|v| v.as_bytes())?;

    match body {
        Body::Text(text) => {
            verify_signature(expected_signature, timestamp, &text, signing_secret)?;
            Ok(text.to_owned())
        }
        _ => bail!("Bad body format"),
    }
}

fn verify_signature(
    expected_signature: &[u8],
    timestamp: &[u8],
    body: &str,
    signing_secret: &str,
) -> Result<(), Error> {
    let verification_key = hmac::VerificationKey::new(&digest::SHA256, signing_secret.as_bytes());
    let timestamp_as_utf8 = str::from_utf8(timestamp)?;
    let payload = format!("v0:{}:{}", timestamp_as_utf8, body);
    let hex_signature = HEXUPPER.decode(expected_signature)?;
    let result = hmac::verify(&verification_key, payload.as_bytes(), &hex_signature);

    if result.is_err() {
        bail!("unauthorized")
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use data_encoding::HEXUPPER;
    use lambda_http::{http, Body};
    use ring::{digest, hmac};

    fn setup_authentication_test() -> (http::request::Builder, lambda_http::Body, String) {
        let signing_secret = "foo";
        let timestamp = "123456789";
        let body = "bar";

        let key = hmac::SigningKey::new(&digest::SHA256, signing_secret.as_bytes());
        let signature = hmac::sign(&key, "v0:123456789:bar".as_bytes());

        let mut builder = http::request::Builder::new();
        builder.header("X-Slack-Request-Timestamp", timestamp);
        builder.header("X-Slack-Signature", HEXUPPER.encode(signature.as_ref()));

        (builder, Body::from(body), signing_secret.to_owned())
    }

    #[test]
    fn test_authenticate_request_works() {
        let (mut builder, body, signing_secret) = setup_authentication_test();

        let request = builder.body(body).unwrap();
        let (parts, body) = request.into_parts();
        let text = authenticate_request(&parts, &body, &signing_secret).unwrap();
        match body {
            Body::Text(expected_text) => assert_eq!(text, expected_text),
            _ => unreachable!(),
        }
    }

    #[test]
    fn test_authenticate_request_missing_timestamp() {
        let (mut builder, body, signing_secret) = setup_authentication_test();
        let headers = builder.headers_mut().unwrap();
        headers.remove("X-Slack-Request-Timestamp");

        let request = builder.body(body).unwrap();
        let (parts, body) = request.into_parts();
        let result = authenticate_request(&parts, &body, &signing_secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_authenticate_request_missing_signature() {
        let (mut builder, body, signing_secret) = setup_authentication_test();
        let headers = builder.headers_mut().unwrap();
        headers.remove("X-Slack-Signature");

        let request = builder.body(body).unwrap();
        let (parts, body) = request.into_parts();
        let result = authenticate_request(&parts, &body, &signing_secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_authenticate_request_bad_signature() {
        let (mut builder, body, signing_secret) = setup_authentication_test();
        let headers = builder.headers_mut().unwrap();
        headers.remove("X-Slack-Signature");
        builder.header("X-Slack-Signature", "elite hacker");

        let request = builder.body(body).unwrap();
        let (parts, body) = request.into_parts();
        let result = authenticate_request(&parts, &body, &signing_secret);
        assert!(result.is_err());
    }

    #[test]
    fn test_authenticate_request_bad_body() {
        let (mut builder, _, signing_secret) = setup_authentication_test();

        let request = builder.body(Body::from("mediocre hacker")).unwrap();
        let (parts, body) = request.into_parts();
        let result = authenticate_request(&parts, &body, &signing_secret);
        assert!(result.is_err());
    }
}

mod slack;

use failure::Error;
use lambda_http::{lambda, IntoResponse, Request};
use lambda_runtime::error::HandlerError;
use lambda_runtime::Context;
use serde_json::json;
use simple_logger;
use std::env;

fn handle_challenge(wrapper: &slack::EventWrapper) -> Result<serde_json::Value, Error> {
    Ok(json!({ "challenge": wrapper.challenge }))
}

fn handle_analysis(wrapper: &slack::EventWrapper) -> Result<serde_json::Value, Error> {
    match wrapper.event.r#type {
        slack::EventType::AppMention => Ok(json!({ "success": true })),
        slack::EventType::Skip => Ok(json!({})),
    }
}

fn handler(request: Request, _: Context) -> Result<impl IntoResponse, HandlerError> {
    let signing_secret = env::var("slack_signing_secret")?;
    let (parts, body) = request.into_parts();
    let text_body = slack::authenticate_request(&parts, &body, &signing_secret)?;

    let wrapper: slack::EventWrapper = serde_json::from_str(&text_body)?;
    let response = match wrapper.r#type {
        slack::EventWrapperType::UrlVerification => handle_challenge(&wrapper)?,
        slack::EventWrapperType::EventCallback => handle_analysis(&wrapper)?,
    };
    Ok(response)
}

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();
    lambda!(handler)
}

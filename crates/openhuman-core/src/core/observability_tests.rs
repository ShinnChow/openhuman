use super::*;

#[cfg(feature = "crash-reporting")]
fn event_with_tags(pairs: &[(&str, &str)]) -> sentry::protocol::Event<'static> {
    let mut event = sentry::protocol::Event::default();
    let mut tags: std::collections::BTreeMap<String, String> =
        std::collections::BTreeMap::new();
    for (k, v) in pairs {
        tags.insert((*k).to_string(), (*v).to_string());
    }
    event.tags = tags;
    event
}

#[cfg(feature = "crash-reporting")]
fn event_with_message(msg: &str) -> sentry::protocol::Event<'static> {
    let mut event = sentry::protocol::Event::default();
    event.message = Some(msg.to_string());
    event
}

#[cfg(feature = "crash-reporting")]
fn channel_message_404_event(method: &str) -> sentry::protocol::Event<'static> {
    let mut event = sentry::protocol::Event::default();
    event.tags.insert("domain".into(), "backend_api".into());
    event.tags.insert("failure".into(), "non_2xx".into());
    event.tags.insert("status".into(), "404".into());
    event.tags.insert("method".into(), method.into());
    event.message = Some(
        "PATCH /channels/telegram/messages/1103 failed (404); response_body_len=172"
            .to_string(),
    );
    event
}

fn managed_body(status: &str, code: &str) -> String {
    format!(
        "OpenHuman API error ({status}): {{\"error\":{{\"errorCode\":\"{code}\",\"message\":\"x\"}}}}"
    )
}

#[cfg(feature = "crash-reporting")]
fn auth_get_me_tags() -> Vec<(&'static str, &'static str)> {
    vec![
        ("domain", "rpc"),
        ("operation", "invoke_method"),
        ("method", "openhuman.auth_get_me"),
        ("elapsed_ms", "5003"),
    ]
}

#[cfg(feature = "crash-reporting")]
fn event_with_tags_and_message(
    pairs: &[(&str, &str)],
    message: &str,
) -> sentry::protocol::Event<'static> {
    let mut event = event_with_tags(pairs);
    event.message = Some(message.to_string());
    event
}

#[cfg(feature = "crash-reporting")]
fn event_with_exception_value(value: &str) -> sentry::protocol::Event<'static> {
    let mut event = sentry::protocol::Event::default();
    event.exception = vec![sentry::protocol::Exception {
        value: Some(value.to_string()),
        ..Default::default()
    }]
    .into();
    event
}


#[path = "observability_tests_part_01.rs"]
mod part_01;
#[path = "observability_tests_part_02.rs"]
mod part_02;
#[path = "observability_tests_part_03.rs"]
mod part_03;
#[path = "observability_tests_part_04.rs"]
mod part_04;
#[path = "observability_tests_part_05.rs"]
mod part_05;
#[path = "observability_tests_part_06.rs"]
mod part_06;
#[path = "observability_tests_part_07.rs"]
mod part_07;
#[path = "observability_tests_part_08.rs"]
mod part_08;

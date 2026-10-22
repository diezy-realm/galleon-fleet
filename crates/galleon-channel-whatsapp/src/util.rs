use galleon_i18n as i18n;

pub use galleon_channel_core::approval::{new_approval_token, parse_approval_reply};

pub const MIN_VOICE_REPLY_BYTES: usize = 40;
pub const MAX_AUDIO_TAG_INNER_BYTES: usize = 30;

#[derive(Debug, Clone, PartialEq)]
pub struct WhatsAppLocation {
    pub lat: f64,
    pub lng: f64,
    pub name: Option<String>,
    pub address: Option<String>,
}

impl WhatsAppLocation {
    pub fn parse(target: &str) -> Option<Self> {
        fn next_field(s: &str) -> (&str, &str) {
            let s = s.trim();
            if let Some(inner) = s.strip_prefix('"') {
                if let Some(end) = inner.find('"') {
                    let field = &inner[..end];
                    let rest = inner[end + 1..].trim();
                    let rest = match rest.strip_prefix(',') {
                        Some(s) => s.trim(),
                        None => "",
                    };
                    return (field, rest);
                }
                return (inner, "");
            }
            match s.find(',') {
                Some(pos) => (s[..pos].trim(), s[pos + 1..].trim()),
                None => (s, ""),
            }
        }

        let (lat_str, rest) = next_field(target);
        let lat: f64 = lat_str.parse().ok()?;
        let (lng_str, rest) = next_field(rest);
        let lng: f64 = lng_str.parse().ok()?;

        if !(-90.0..=90.0).contains(&lat) || !(-180.0..=180.0).contains(&lng) {
            return None;
        }

        let (name_raw, rest) = next_field(rest);
        let name = (!name_raw.is_empty()).then(|| name_raw.to_string());
        let address = (!rest.is_empty()).then(|| rest.to_string());

        Some(Self {
            lat,
            lng,
            name,
            address,
        })
    }
}

pub fn format_location_content(lat: f64, lng: f64, name: Option<&str>) -> String {
    match name.filter(|n| !n.is_empty()) {
        Some(name) => format!("[Location: {lat:.6}, {lng:.6} — {name}]"),
        None => format!("[Location: {lat:.6}, {lng:.6}]"),
    }
}

const ATTACHMENT_KINDS: &[&str] = &[
    "IMAGE", "PHOTO", "DOCUMENT", "FILE", "VIDEO", "AUDIO", "VOICE", "LOCATION",
];

pub fn parse_attachment_markers(message: &str) -> (String, Vec<(String, String)>) {
    parse_attachment_markers_of_kinds(message, ATTACHMENT_KINDS)
}

pub fn parse_attachment_markers_of_kinds(
    message: &str,
    kinds: &[&str],
) -> (String, Vec<(String, String)>) {
    let mut cleaned = String::with_capacity(message.len());
    let mut attachments = Vec::new();
    let mut cursor = 0usize;

    while let Some(rel_start) = message[cursor..].find('[') {
        let start = cursor + rel_start;
        cleaned.push_str(&message[cursor..start]);

        let Some(rel_end) = message[start..].find(']') else {
            cleaned.push_str(&message[start..]);
            cursor = message.len();
            break;
        };
        let end = start + rel_end;
        let marker_text = &message[start + 1..end];

        let parsed = marker_text.split_once(':').and_then(|(kind, target)| {
            let kind_upper = kind.trim().to_ascii_uppercase();
            let target = target.trim();
            if target.is_empty() || !kinds.contains(&kind_upper.as_str()) {
                return None;
            }
            Some((kind_upper, target.to_string()))
        });

        if let Some((kind, target)) = parsed {
            attachments.push((kind, target));
        } else {
            cleaned.push_str(&message[start..=end]);
        }
        cursor = end + 1;
    }

    if cursor < message.len() {
        cleaned.push_str(&message[cursor..]);
    }

    (cleaned.trim().to_string(), attachments)
}

pub fn approval_position_line(position: Option<(u32, u32)>) -> String {
    match position {
        Some((_, total)) if total <= 1 => String::new(),
        Some((index, total)) => {
            let line = i18n::get_required_cli_string_with_args(
                "channel-approval-position",
                &[
                    ("index", index.to_string().as_str()),
                    ("total", total.to_string().as_str()),
                ],
            );
            format!("{line}\n")
        }
        None => String::new(),
    }
}

pub fn build_yesno_approval_prompt(
    token: &str,
    tool_name: &str,
    arguments_summary: &str,
    position: Option<(u32, u32)>,
) -> String {
    let position_line = approval_position_line(position);
    let heading = i18n::get_required_cli_string("channel-approval-heading-shout");
    let tool_label = i18n::get_required_cli_string("channel-approval-tool-label");
    let args_label = i18n::get_required_cli_string("channel-approval-args-label");
    let yes_command = format!("{token} yes");
    let no_command = format!("{token} no");
    let always_command = format!("{token} always");
    let reply = i18n::get_required_cli_string_with_args(
        "channel-approval-reply-instruction-yesno",
        &[
            ("yes_command", yes_command.as_str()),
            ("no_command", no_command.as_str()),
            ("always_command", always_command.as_str()),
        ],
    );

    format!("{heading} [{token}]\n{position_line}{tool_label}: {tool_name}\n{args_label}: {arguments_summary}\n\n{reply}")
}

pub fn strip_tool_call_tags(message: &str) -> String {
    const TOOL_CALL_OPEN_TAGS: [&str; 7] = [
        "<function_calls>",
        "<function_call>",
        "<tool_call>",
        "<toolcall>",
        "<tool-call>",
        "<tool>",
        "<invoke>",
    ];

    fn find_first_tag<'a>(haystack: &str, tags: &'a [&'a str]) -> Option<(usize, &'a str)> {
        tags.iter()
            .filter_map(|tag| haystack.find(tag).map(|idx| (idx, *tag)))
            .min_by_key(|(idx, _)| *idx)
    }

    fn matching_close_tag(open_tag: &str) -> Option<&'static str> {
        match open_tag {
            "<function_calls>" => Some("</function_calls>"),
            "<function_call>" => Some("</function_call>"),
            "<tool_call>" => Some("</tool_call>"),
            "<toolcall>" => Some("</toolcall>"),
            "<tool-call>" => Some("</tool-call>"),
            "<tool>" => Some("</tool>"),
            "<invoke>" => Some("</invoke>"),
            _ => None,
        }
    }

    let mut remaining = message;
    let mut kept_segments = Vec::new();

    while let Some((start, open_tag)) = find_first_tag(remaining, &TOOL_CALL_OPEN_TAGS) {
        kept_segments.push(remaining[..start].to_string());
        let after_open = &remaining[start + open_tag.len()..];

        if let Some(close_tag) = matching_close_tag(open_tag)
            && let Some(close_idx) = after_open.find(close_tag)
        {
            remaining = &after_open[close_idx + close_tag.len()..];
            continue;
        }

        kept_segments.push(remaining[start..].to_string());
        remaining = "";
        break;
    }

    if !remaining.is_empty() {
        kept_segments.push(remaining.to_string());
    }

    let mut result = kept_segments.concat();
    while result.contains("\n\n\n") {
        result = result.replace("\n\n\n", "\n\n");
    }

    result.trim().to_string()
}

fn leading_bracket_skip_reason(content: &str) -> Option<&'static str> {
    let after_open = content.strip_prefix('[')?;

    if after_open.starts_with(|c: char| matches!(c, '{' | '"' | ']') || c.is_ascii_digit()) {
        return Some("json_array");
    }

    let Some(close) = after_open.find(']') else {
        return Some("unclosed_bracket");
    };
    let inner = &after_open[..close];
    let tail = &after_open[close + 1..];

    if inner.contains(':') {
        return Some("attachment_marker");
    }
    if tail.starts_with('(') {
        return Some("markdown_link");
    }
    if inner.len() > MAX_AUDIO_TAG_INNER_BYTES {
        return Some("bracketed_prefix");
    }

    None
}

pub fn voice_reply_skip_reason(content: &str) -> Option<&'static str> {
    if content.len() <= MIN_VOICE_REPLY_BYTES {
        return Some("too_short");
    }
    if content.starts_with("http") {
        return Some("url_prefix");
    }
    if content.starts_with('{') {
        return Some("json_object");
    }
    if content.starts_with("```") {
        return Some("code_block");
    }
    if content.starts_with('[') {
        return leading_bracket_skip_reason(content);
    }
    None
}

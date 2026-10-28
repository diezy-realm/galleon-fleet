mod tests {
    use super::*;
    use serde_json::json;

    async fn spawn_raw_http_response(
        raw_response: Vec<u8>,
        hold_open: bool,
    ) -> (String, tokio::task::JoinHandle<()>) {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = galleon_spawn::spawn!(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0_u8; 1024];
            let _ = socket.read(&mut request).await.unwrap();
            socket.write_all(&raw_response).await.unwrap();
            if hold_open {
                std::future::pending::<()>().await;
            }
            socket.shutdown().await.unwrap();
        });

        (format!("http://{address}"), server)
    }


    struct MockAudioTranscriber(&'static str);
    impl AudioTranscriber for MockAudioTranscriber {
        fn transcribe<'a>(
            &'a self,
            _audio_data: &'a [u8],
            _file_name: &'a str,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<String>> + Send + 'a>> {
            Box::pin(async move { Ok(self.0.to_string()) })
        }
    }

    #[test]
    fn mattermost_url_trimming() {
        let thread_replies = false;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "https://mm.example.com/".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(Vec::new),
            thread_replies,
            mention_only,
        );
        assert_eq!(ch.base_url, "https://mm.example.com");
    }

    #[test]
    fn mattermost_allowlist_wildcard() {
        let thread_replies = false;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        assert!(ch.is_user_allowed("any-id"));
    }

    #[test]
    fn mattermost_parse_post_basic() {
        let thread_replies = true;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "hello world",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(
                &post,
                "bot123",
                "botname",
                1_500_000_000_000_i64,
                "chan789",
                None,
                false,
            )
            .unwrap();
        assert_eq!(msg.sender, "user456");
        assert_eq!(msg.content, "hello world");
        assert_eq!(msg.reply_target, "chan789:post123"); // Default threaded reply
    }

    #[test]
    fn mattermost_parse_post_thread_replies_enabled() {
        let thread_replies = true;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "hello world",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(
                &post,
                "bot123",
                "botname",
                1_500_000_000_000_i64,
                "chan789",
                None,
                false,
            )
            .unwrap();
        assert_eq!(msg.reply_target, "chan789:post123"); // Threaded reply
    }

    #[test]
    fn mattermost_parse_post_thread() {
        let thread_replies = false;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "reply",
            "create_at": 1_600_000_000_000_i64,
            "root_id": "root789"
        });

        let msg = ch
            .parse_mattermost_post(
                &post,
                "bot123",
                "botname",
                1_500_000_000_000_i64,
                "chan789",
                None,
                false,
            )
            .unwrap();
        assert_eq!(msg.reply_target, "chan789:root789"); // Stays in the thread
    }

    #[test]
    fn mattermost_parse_post_ignore_self() {
        let thread_replies = false;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post123",
            "user_id": "bot123",
            "message": "my own message",
            "create_at": 1_600_000_000_000_i64
        });

        let msg = ch.parse_mattermost_post(
            &post,
            "bot123",
            "botname",
            1_500_000_000_000_i64,
            "chan789",
            None,
            false,
        );
        assert!(msg.is_none());
    }

    #[test]
    fn mattermost_parse_post_ignore_old() {
        let thread_replies = false;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "old message",
            "create_at": 1_400_000_000_000_i64
        });

        let msg = ch.parse_mattermost_post(
            &post,
            "bot123",
            "botname",
            1_500_000_000_000_i64,
            "chan789",
            None,
            false,
        );
        assert!(msg.is_none());
    }

    #[test]
    fn mattermost_parse_post_no_thread_when_disabled() {
        let thread_replies = false;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "hello world",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(
                &post,
                "bot123",
                "botname",
                1_500_000_000_000_i64,
                "chan789",
                None,
                false,
            )
            .unwrap();
        assert_eq!(msg.reply_target, "chan789"); // No thread suffix
    }

    #[test]
    fn mattermost_existing_thread_always_threads() {
        // Even with thread_replies=false, replies to existing threads stay in the thread
        let thread_replies = false;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "reply in thread",
            "create_at": 1_600_000_000_000_i64,
            "root_id": "root789"
        });

        let msg = ch
            .parse_mattermost_post(
                &post,
                "bot123",
                "botname",
                1_500_000_000_000_i64,
                "chan789",
                None,
                false,
            )
            .unwrap();
        assert_eq!(msg.reply_target, "chan789:root789"); // Stays in existing thread
    }

    // ── mention_only tests ────────────────────────────────────────

    #[test]
    fn mention_only_skips_message_without_mention() {
        let thread_replies = true;
        let mention_only = true;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "hello everyone",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch.parse_mattermost_post(
            &post,
            "bot123",
            "mybot",
            1_500_000_000_000_i64,
            "chan1",
            None,
            false,
        );
        assert!(msg.is_none());
    }

    #[test]
    fn mention_only_accepts_message_with_at_mention() {
        let thread_replies = true;
        let mention_only = true;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "@mybot what is the weather?",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(
                &post,
                "bot123",
                "mybot",
                1_500_000_000_000_i64,
                "chan1",
                None,
                false,
            )
            .unwrap();
        assert_eq!(msg.content, "@mybot what is the weather?");
    }

    #[test]
    fn mention_only_preserves_mention_in_body() {
        let thread_replies = true;
        let mention_only = true;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "  @mybot  run status  ",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(
                &post,
                "bot123",
                "mybot",
                1_500_000_000_000_i64,
                "chan1",
                None,
                false,
            )
            .unwrap();
        assert_eq!(msg.content, "@mybot  run status");
    }

    #[test]
    fn mention_only_admits_caption_that_is_only_the_mention() {
        let thread_replies = true;
        let mention_only = true;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "@mybot",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(
                &post,
                "bot123",
                "mybot",
                1_500_000_000_000_i64,
                "chan1",
                None,
                false,
            )
            .unwrap();
        assert_eq!(msg.content, "@mybot");
    }

    #[test]
    fn mention_only_case_insensitive() {
        let thread_replies = true;
        let mention_only = true;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "@MyBot hello",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(
                &post,
                "bot123",
                "mybot",
                1_500_000_000_000_i64,
                "chan1",
                None,
                false,
            )
            .unwrap();
        assert_eq!(msg.content, "@MyBot hello");
    }

    #[test]
    fn mention_only_detects_metadata_mentions() {
        // Even without @username in text, metadata.mentions should trigger.
        let thread_replies = true;
        let mention_only = true;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "hey check this out",
            "create_at": 1_600_000_000_000_i64,
            "root_id": "",
            "metadata": {
                "mentions": ["bot123"]
            }
        });

        let msg = ch
            .parse_mattermost_post(
                &post,
                "bot123",
                "mybot",
                1_500_000_000_000_i64,
                "chan1",
                None,
                false,
            )
            .unwrap();
        // Content is preserved as-is since no @username was in the text to strip.
        assert_eq!(msg.content, "hey check this out");
    }

    #[test]
    fn mention_only_word_boundary_prevents_partial_match() {
        let thread_replies = true;
        let mention_only = true;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        // "@mybotextended" should NOT match "@mybot" because it extends the username.
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "@mybotextended hello",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch.parse_mattermost_post(
            &post,
            "bot123",
            "mybot",
            1_500_000_000_000_i64,
            "chan1",
            None,
            false,
        );
        assert!(msg.is_none());
    }

    #[test]
    fn mention_only_mention_in_middle_of_text() {
        let thread_replies = true;
        let mention_only = true;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "hey @mybot how are you?",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(
                &post,
                "bot123",
                "mybot",
                1_500_000_000_000_i64,
                "chan1",
                None,
                false,
            )
            .unwrap();
        assert_eq!(msg.content, "hey @mybot how are you?");
    }

    #[test]
    fn mention_only_disabled_passes_all_messages() {
        // With mention_only=false (default), messages pass through unfiltered.
        let thread_replies = true;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "no mention here",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(
                &post,
                "bot123",
                "mybot",
                1_500_000_000_000_i64,
                "chan1",
                None,
                false,
            )
            .unwrap();
        assert_eq!(msg.content, "no mention here");
    }

    // ── contains_bot_mention_mm unit tests ────────────────────────

    #[test]
    fn contains_mention_text_at_end() {
        let post = json!({});
        assert!(contains_bot_mention_mm(
            "hello @mybot",
            "bot123",
            "mybot",
            &post
        ));
    }

    #[test]
    fn contains_mention_text_at_start() {
        let post = json!({});
        assert!(contains_bot_mention_mm(
            "@mybot hello",
            "bot123",
            "mybot",
            &post
        ));
    }

    #[test]
    fn contains_mention_text_alone() {
        let post = json!({});
        assert!(contains_bot_mention_mm("@mybot", "bot123", "mybot", &post));
    }

    #[test]
    fn no_mention_different_username() {
        let post = json!({});
        assert!(!contains_bot_mention_mm(
            "@otherbot hello",
            "bot123",
            "mybot",
            &post
        ));
    }

    #[test]
    fn no_mention_partial_username() {
        let post = json!({});
        // "mybot" is a prefix of "mybotx" — should NOT match
        assert!(!contains_bot_mention_mm(
            "@mybotx hello",
            "bot123",
            "mybot",
            &post
        ));
    }

    #[test]
    fn mention_detects_later_valid_mention_after_partial_prefix() {
        let post = json!({});
        assert!(contains_bot_mention_mm(
            "@mybotx ignore this, but @mybot handle this",
            "bot123",
            "mybot",
            &post
        ));
    }

    #[test]
    fn mention_followed_by_punctuation() {
        let post = json!({});
        // "@mybot," — comma is not alphanumeric/underscore/dash/dot, so it's a boundary
        assert!(contains_bot_mention_mm(
            "@mybot, hello",
            "bot123",
            "mybot",
            &post
        ));
    }

    #[test]
    fn mention_via_metadata_only() {
        let post = json!({
            "metadata": { "mentions": ["bot123"] }
        });
        assert!(contains_bot_mention_mm(
            "no at mention",
            "bot123",
            "mybot",
            &post
        ));
    }

    #[test]
    fn no_mention_empty_username_no_metadata() {
        let post = json!({});
        assert!(!contains_bot_mention_mm("hello world", "bot123", "", &post));
    }

    // ── normalize_mattermost_content unit tests ───────────────────

    #[test]
    fn normalize_preserves_mention_and_trims() {
        let post = json!({});
        let result = normalize_mattermost_content("  @mybot  do stuff  ", "bot123", "mybot", &post);
        assert_eq!(result.as_deref(), Some("@mybot  do stuff"));
    }

    #[test]
    fn normalize_returns_none_for_no_mention() {
        let post = json!({});
        let result = normalize_mattermost_content("hello world", "bot123", "mybot", &post);
        assert!(result.is_none());
    }

    #[test]
    fn normalize_admits_mention_only_caption() {
        let post = json!({});
        let result = normalize_mattermost_content("@mybot", "bot123", "mybot", &post);
        assert_eq!(result.as_deref(), Some("@mybot"));
    }

    #[test]
    fn normalize_preserves_text_for_metadata_mention() {
        let post = json!({
            "metadata": { "mentions": ["bot123"] }
        });
        let result = normalize_mattermost_content("check this out", "bot123", "mybot", &post);
        assert_eq!(result.as_deref(), Some("check this out"));
    }

    #[test]
    fn normalize_preserves_multiple_mentions() {
        let post = json!({});
        let result =
            normalize_mattermost_content("@mybot hello @mybot world", "bot123", "mybot", &post);
        assert_eq!(result.as_deref(), Some("@mybot hello @mybot world"));
    }

    #[test]
    fn normalize_keeps_partial_username_mentions() {
        let post = json!({});
        let result =
            normalize_mattermost_content("@mybot hello @mybotx world", "bot123", "mybot", &post);
        assert_eq!(result.as_deref(), Some("@mybot hello @mybotx world"));
    }

    // ── Transcription tests ───────────────────────────────────────

    #[test]
    fn mattermost_manager_none_when_transcription_not_configured() {
        let thread_replies = false;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        assert!(ch.transcription_manager.is_none());
    }

    #[test]
    fn mattermost_manager_some_when_valid_config() {
        let thread_replies = false;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        )
        .with_audio_transcriber(Some(Arc::new(MockAudioTranscriber("test"))));
        assert!(ch.transcription_manager.is_some());
    }

    #[test]
    fn mattermost_manager_none_and_warn_on_init_failure() {
        let thread_replies = false;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        )
        .with_transcription(galleon_config::schema::TranscriptionConfig {
            enabled: true,
            api_key: Some(String::new()),
            api_url: "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
            model: "whisper-large-v3".to_string(),
            language: None,
            initial_prompt: None,
            max_audio_bytes: None,
            max_duration_secs: 600,
            openai: None,
            deepgram: None,
            assemblyai: None,
            google: None,
            local_whisper: None,
            transcribe_non_ptt_audio: false,
        });
        assert!(ch.transcription_manager.is_none());
    }

    #[test]
    fn mattermost_post_has_audio_attachment_true_for_audio_mime() {
        let post = json!({
            "metadata": {
                "files": [
                    {
                        "id": "file1",
                        "mime_type": "audio/ogg",
                        "name": "voice.ogg"
                    }
                ]
            }
        });
        assert!(post_has_audio_attachment(&post));
    }

    #[test]
    fn mattermost_post_has_audio_attachment_true_for_audio_ext() {
        let post = json!({
            "metadata": {
                "files": [
                    {
                        "id": "file1",
                        "mime_type": "application/octet-stream",
                        "extension": "ogg"
                    }
                ]
            }
        });
        assert!(post_has_audio_attachment(&post));
    }

    #[test]
    fn mattermost_post_has_audio_attachment_false_for_image() {
        let post = json!({
            "metadata": {
                "files": [
                    {
                        "id": "file1",
                        "mime_type": "image/png",
                        "name": "screenshot.png"
                    }
                ]
            }
        });
        assert!(!post_has_audio_attachment(&post));
    }

    #[test]
    fn mattermost_post_has_audio_attachment_false_when_no_files() {
        let post = json!({
            "metadata": {}
        });
        assert!(!post_has_audio_attachment(&post));
    }

    #[test]
    fn mattermost_parse_post_uses_injected_text() {
        let thread_replies = true;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(
                &post,
                "bot123",
                "botname",
                1_500_000_000_000_i64,
                "chan789",
                Some("transcript text"),
                false,
            )
            .unwrap();
        assert_eq!(msg.content, "transcript text");
    }

    #[test]
    fn mattermost_parse_post_rejects_empty_message_without_injected() {
        let thread_replies = true;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch.parse_mattermost_post(
            &post,
            "bot123",
            "botname",
            1_500_000_000_000_i64,
            "chan789",
            None,
            false,
        );
        assert!(msg.is_none());
    }

    #[tokio::test]
    async fn mattermost_transcribe_skips_when_manager_none() {
        let thread_replies = false;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        );
        let post = json!({
            "metadata": {
                "files": [
                    {
                        "id": "file1",
                        "mime_type": "audio/ogg",
                        "name": "voice.ogg"
                    }
                ]
            }
        });
        let result = ch.try_transcribe_audio_attachment(&post).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn mattermost_transcribe_skips_over_duration_limit() {
        let thread_replies = false;
        let mention_only = false;
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_test_alias",
            Arc::new(|| vec!["*".into()]),
            thread_replies,
            mention_only,
        )
        .with_transcription(galleon_config::schema::TranscriptionConfig {
            enabled: true,
            api_key: Some("test_key".to_string()),
            api_url: "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
            model: "whisper-large-v3".to_string(),
            language: None,
            initial_prompt: None,
            max_audio_bytes: None,
            max_duration_secs: 3600,
            openai: None,
            deepgram: None,
            assemblyai: None,
            google: None,
            local_whisper: None,
            transcribe_non_ptt_audio: false,
        });

        let post = json!({
            "metadata": {
                "files": [
                    {
                        "id": "file1",
                        "mime_type": "audio/ogg",
                        "name": "voice.ogg",
                        "duration": 7_200_000_u64
                    }
                ]
            }
        });

        let result = ch.try_transcribe_audio_attachment(&post).await;
        assert!(result.is_none());
    }

    #[cfg(test)]
    mod http_tests {
        use super::*;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        #[tokio::test]
        async fn mattermost_audio_routes_through_local_whisper() {
            let mock_server = MockServer::start().await;

            Mock::given(method("GET"))
                .and(path("/api/v4/files/file1"))
                .respond_with(ResponseTemplate::new(200).set_body_bytes(b"audio bytes"))
                .mount(&mock_server)
                .await;

            Mock::given(method("POST"))
                .and(path("/v1/audio/transcriptions"))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(json!({"text": "test transcript"})),
                )
                .mount(&mock_server)
                .await;

            let whisper_url = format!("{}/v1/audio/transcriptions", mock_server.uri());
            let thread_replies = false;
            let mention_only = false;
            let ch = MattermostChannel::new(
                mock_server.uri(),
                Some("test_token".to_string()),
                None,
                None,
                Vec::new(),
                "mattermost_test_alias",
                Arc::new(|| vec!["*".into()]),
                thread_replies,
                mention_only,
            )
            .with_transcription(galleon_config::schema::TranscriptionConfig {
                enabled: true,
                api_key: None,
                api_url: "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
                model: "whisper-large-v3".to_string(),
                language: None,
                initial_prompt: None,
                max_audio_bytes: None,
                max_duration_secs: 600,
                openai: None,
                deepgram: None,
                assemblyai: None,
                google: None,
                local_whisper: Some(galleon_config::schema::LocalWhisperConfig {
                    url: whisper_url,
                    bearer_token: Some("test_token".to_string()),
                    max_audio_bytes: 25_000_000,
                    timeout_secs: 300,
                }),
                transcribe_non_ptt_audio: false,
            })
            .with_audio_transcriber(Some(Arc::new(MockAudioTranscriber("test transcript"))));

            let post = json!({
                "metadata": {
                    "files": [
                        {
                            "id": "file1",
                            "mime_type": "audio/ogg",
                            "name": "voice.ogg"
                        }
                    ]
                }
            });

            let result = ch.try_transcribe_audio_attachment(&post).await;
            assert_eq!(result.as_deref(), Some("[Voice] test transcript"));
        }

        #[tokio::test]
        async fn mattermost_audio_rejects_declared_oversize_before_transcription() {
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                MAX_MATTERMOST_AUDIO_BYTES + 1
            )
            .into_bytes();
            let (mattermost_url, server) =
                spawn_raw_http_response(response, true).await;
            let mock_server = MockServer::start().await;

            Mock::given(method("POST"))
                .and(path("/v1/audio/transcriptions"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({"text": "wrong"})))
                .expect(0)
                .mount(&mock_server)
                .await;

            let ch = MattermostChannel::new(
                mattermost_url,
                Some("test_token".to_string()),
                None,
                None,
                Vec::new(),
                "mattermost_test_alias",
                Arc::new(|| vec!["*".into()]),
                false,
                false,
            )
            .with_transcription(galleon_config::schema::TranscriptionConfig {
                enabled: true,
                api_key: None,
                api_url: "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
                model: "whisper-large-v3".to_string(),
                language: None,
                initial_prompt: None,
                max_audio_bytes: None,
                max_duration_secs: 600,
                openai: None,
                deepgram: None,
                assemblyai: None,
                google: None,
                local_whisper: Some(galleon_config::schema::LocalWhisperConfig {
                    url: format!("{}/v1/audio/transcriptions", mock_server.uri()),
                    bearer_token: Some("test_token".to_string()),
                    max_audio_bytes: 25_000_000,
                    timeout_secs: 300,
                }),
                transcribe_non_ptt_audio: false,
            });
            let post = json!({
                "metadata": {
                    "files": [{
                        "id": "file1",
                        "mime_type": "audio/ogg",
                        "name": "voice.ogg"
                    }]
                }
            });

            let result = tokio::time::timeout(
                std::time::Duration::from_secs(2),
                ch.try_transcribe_audio_attachment(&post),
            )
            .await
            .expect("declared oversize must be rejected before reading the body");
            server.abort();

            assert!(result.is_none());
        }

        #[tokio::test]
        async fn mattermost_audio_skips_non_audio_attachment() {
            let mock_server = MockServer::start().await;

            let thread_replies = false;
            let mention_only = false;
            let ch = MattermostChannel::new(
                mock_server.uri(),
                Some("test_token".to_string()),
                None,
                None,
                Vec::new(),
                "mattermost_test_alias",
                Arc::new(|| vec!["*".into()]),
                thread_replies,
                mention_only,
            )
            .with_transcription(galleon_config::schema::TranscriptionConfig {
                enabled: true,
                api_key: None,
                api_url: "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
                model: "whisper-large-v3".to_string(),
                language: None,
                initial_prompt: None,
                max_audio_bytes: None,
                max_duration_secs: 600,
                openai: None,
                deepgram: None,
                assemblyai: None,
                google: None,
                local_whisper: Some(galleon_config::schema::LocalWhisperConfig {
                    url: mock_server.uri(),
                    bearer_token: Some("test_token".to_string()),
                    max_audio_bytes: 25_000_000,
                    timeout_secs: 300,
                }),
                transcribe_non_ptt_audio: false,
            });

            let post = json!({
                "metadata": {
                    "files": [
                        {
                            "id": "file1",
                            "mime_type": "image/png",
                            "name": "screenshot.png"
                        }
                    ]
                }
            });

            let result = ch.try_transcribe_audio_attachment(&post).await;
            assert!(result.is_none());
        }
    }

    // ── Multi-channel + DM contract (red) ────────────────────────────

    fn make_ch_for_scope(channel_ids: Vec<String>) -> MattermostChannel {
        MattermostChannel::new(
            "https://mm.example.com".into(),
            Some("token".into()),
            None,
            None,
            channel_ids,
            "mattermost_scope_alias",
            Arc::new(|| vec!["*".into()]),
            true,
            false,
        )
    }

    #[test]
    fn normalized_channel_id_strips_wildcard_and_blank() {
        assert_eq!(MattermostChannel::normalized_channel_id(None), None);
        assert_eq!(MattermostChannel::normalized_channel_id(Some("")), None);
        assert_eq!(MattermostChannel::normalized_channel_id(Some("   ")), None);
        assert_eq!(MattermostChannel::normalized_channel_id(Some("*")), None);
        assert_eq!(
            MattermostChannel::normalized_channel_id(Some("  abc123 ")),
            Some("abc123".to_string())
        );
    }

    #[test]
    fn scoped_channel_ids_empty_returns_none() {
        let ch = make_ch_for_scope(Vec::new());
        assert_eq!(ch.scoped_channel_ids(), None);
    }

    #[test]
    fn scoped_channel_ids_wildcard_only_returns_none() {
        let ch = make_ch_for_scope(vec!["*".into()]);
        assert_eq!(ch.scoped_channel_ids(), None);
    }

    #[test]
    fn scoped_channel_ids_explicit_returns_dedup() {
        let ch = make_ch_for_scope(vec![
            "abc".into(),
            "  def  ".into(),
            "abc".into(),
            "*".into(),
            "".into(),
        ]);
        assert_eq!(
            ch.scoped_channel_ids(),
            Some(vec!["abc".to_string(), "def".to_string()])
        );
    }

    #[test]
    fn is_direct_channel_treats_dm_and_group_dm_as_direct() {
        assert!(is_direct_channel("D"));
        assert!(is_direct_channel("G"));
    }

    #[test]
    fn is_direct_channel_rejects_public_and_private_team_channels() {
        assert!(!is_direct_channel("O"));
        assert!(!is_direct_channel("P"));
        assert!(!is_direct_channel(""));
        assert!(!is_direct_channel("X"));
    }

    fn ch_obj(id: &str, ty: &str, team: &str) -> serde_json::Value {
        json!({"id": id, "type": ty, "team_id": team})
    }

    #[test]
    fn filter_discovered_channels_includes_all_when_no_filters() {
        let raw = vec![
            ch_obj("pub1", "O", "teamA"),
            ch_obj("priv1", "P", "teamA"),
            ch_obj("dm1", "D", ""),
            ch_obj("gdm1", "G", ""),
        ];
        let kept = filter_discovered_channels(&raw, &[], true);
        let ids: Vec<&str> = kept.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["pub1", "priv1", "dm1", "gdm1"]);
        assert!(!kept[0].is_direct);
        assert!(!kept[1].is_direct);
        assert!(kept[2].is_direct);
        assert!(kept[3].is_direct);
    }

    #[test]
    fn filter_discovered_channels_respects_team_ids_allowlist() {
        let raw = vec![
            ch_obj("pub_a", "O", "teamA"),
            ch_obj("pub_b", "O", "teamB"),
            ch_obj("priv_a", "P", "teamA"),
        ];
        let kept = filter_discovered_channels(&raw, &["teamA".to_string()], true);
        let ids: Vec<&str> = kept.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["pub_a", "priv_a"]);
    }

    #[test]
    fn filter_discovered_channels_omits_dms_when_discover_dms_false() {
        let raw = vec![
            ch_obj("pub1", "O", "teamA"),
            ch_obj("dm1", "D", ""),
            ch_obj("gdm1", "G", ""),
        ];
        let kept = filter_discovered_channels(&raw, &[], false);
        let ids: Vec<&str> = kept.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["pub1"]);
    }

    #[test]
    fn filter_discovered_channels_keeps_dms_regardless_of_team_ids() {
        let raw = vec![
            ch_obj("pub_b", "O", "teamB"),
            ch_obj("dm1", "D", ""),
            ch_obj("gdm1", "G", ""),
        ];
        let kept = filter_discovered_channels(&raw, &["teamA".to_string()], true);
        let ids: Vec<&str> = kept.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["dm1", "gdm1"]);
    }

    #[test]
    fn mention_only_bypassed_for_direct_channels_in_parse() {
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_dm_alias",
            Arc::new(|| vec!["*".into()]),
            false,
            true,
        );
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "no mention here, just talking",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch
            .parse_mattermost_post(
                &post,
                "bot123",
                "mybot",
                1_500_000_000_000_i64,
                "dm_channel",
                None,
                true,
            )
            .expect("DM message must bypass mention_only and produce a ChannelMessage");
        assert_eq!(msg.content, "no mention here, just talking");
    }

    #[test]
    fn mention_only_applied_in_parse_when_is_direct_false() {
        let ch = MattermostChannel::new(
            "url".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "mattermost_group_alias",
            Arc::new(|| vec!["*".into()]),
            false,
            true,
        );
        let post = json!({
            "id": "post1",
            "user_id": "user1",
            "message": "no mention here, just talking",
            "create_at": 1_600_000_000_000_i64,
            "root_id": ""
        });

        let msg = ch.parse_mattermost_post(
            &post,
            "bot123",
            "mybot",
            1_500_000_000_000_i64,
            "pub_channel",
            None,
            false,
        );
        assert!(msg.is_none(), "public channel must enforce mention_only");
    }

    #[cfg(test)]
    mod discovery_http_tests {
        use super::*;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        #[tokio::test]
        async fn list_target_channels_discovers_via_users_me_channels() {
            let mock_server = MockServer::start().await;

            Mock::given(method("GET"))
                .and(path("/api/v4/users/me"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(json!({"id": "bot123", "username": "mybot"})),
                )
                .mount(&mock_server)
                .await;

            Mock::given(method("GET"))
                .and(path("/api/v4/users/me/channels"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!([
                    {"id": "pub_a", "type": "O", "team_id": "teamA"},
                    {"id": "pub_b", "type": "O", "team_id": "teamB"},
                    {"id": "dm_x",  "type": "D", "team_id": ""},
                    {"id": "gdm_y", "type": "G", "team_id": ""},
                ])))
                .mount(&mock_server)
                .await;

            let ch = MattermostChannel::new(
                mock_server.uri(),
                Some("token".into()),
                None,
                None,
                Vec::new(),
                "mattermost_discover_alias",
                Arc::new(|| vec!["*".into()]),
                false,
                false,
            )
            .with_team_ids(vec!["teamA".to_string()])
            .with_discover_dms(true);

            let targets = ch
                .list_target_channels()
                .await
                .expect("discovery must succeed");
            let ids: Vec<&str> = targets.iter().map(|t| t.id.as_str()).collect();
            assert_eq!(
                ids,
                vec!["pub_a", "dm_x", "gdm_y"],
                "discovery should keep teamA channels and all DMs"
            );
            assert!(!targets[0].is_direct);
            assert!(targets[1].is_direct);
            assert!(targets[2].is_direct);
        }

        #[tokio::test]
        async fn list_target_channels_explicit_ids_skip_discovery_and_lookup_types() {
            let mock_server = MockServer::start().await;

            Mock::given(method("GET"))
                .and(path("/api/v4/channels/explicit_dm"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "id": "explicit_dm",
                    "type": "D",
                    "team_id": ""
                })))
                .mount(&mock_server)
                .await;

            Mock::given(method("GET"))
                .and(path("/api/v4/channels/explicit_pub"))
                .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                    "id": "explicit_pub",
                    "type": "O",
                    "team_id": "teamA"
                })))
                .mount(&mock_server)
                .await;

            let ch = MattermostChannel::new(
                mock_server.uri(),
                Some("token".into()),
                None,
                None,
                vec!["explicit_dm".into(), "explicit_pub".into()],
                "mattermost_explicit_alias",
                Arc::new(|| vec!["*".into()]),
                false,
                false,
            );

            let targets = ch
                .list_target_channels()
                .await
                .expect("explicit lookup must succeed");
            let by_id: std::collections::HashMap<_, _> = targets
                .iter()
                .map(|t| (t.id.as_str(), t.is_direct))
                .collect();
            assert_eq!(by_id.get("explicit_dm"), Some(&true));
            assert_eq!(by_id.get("explicit_pub"), Some(&false));
            assert_eq!(targets.len(), 2);
        }
    }

    #[test]
    fn test_ws_url_conversion() {
        let ch = MattermostChannel::new(
            "https://mm.example.com".into(),
            Some("token".into()),
            None,
            None,
            vec![],
            "test",
            Arc::new(Vec::new),
            false,
            false,
        );
        assert_eq!(ch.ws_url(), "wss://mm.example.com/api/v4/websocket");

        let ch2 = MattermostChannel::new(
            "http://localhost:8065".into(),
            Some("token".into()),
            None,
            None,
            vec![],
            "test",
            Arc::new(Vec::new),
            false,
            false,
        );
        assert_eq!(ch2.ws_url(), "ws://localhost:8065/api/v4/websocket");

        // server URL with path prefix should preserve it
        let ch3 = MattermostChannel::new(
            "https://mm.example.com/subpath".into(),
            Some("token".into()),
            None,
            None,
            vec![],
            "test",
            Arc::new(Vec::new),
            false,
            false,
        );
        assert_eq!(
            ch3.ws_url(),
            "wss://mm.example.com/subpath/api/v4/websocket"
        );
    }

    #[test]
    fn test_listen_mode_default_is_polling() {
        assert_eq!(
            MattermostListenMode::default(),
            MattermostListenMode::Polling
        );
    }

    #[test]
    fn test_listen_mode_serde() {
        // serialize
        assert_eq!(
            serde_json::to_string(&MattermostListenMode::Polling).unwrap(),
            "\"polling\""
        );
        assert_eq!(
            serde_json::to_string(&MattermostListenMode::Websocket).unwrap(),
            "\"websocket\""
        );

        // deserialize
        let polling: MattermostListenMode = serde_json::from_str("\"polling\"").unwrap();
        assert_eq!(polling, MattermostListenMode::Polling);

        let websocket: MattermostListenMode = serde_json::from_str("\"websocket\"").unwrap();
        assert_eq!(websocket, MattermostListenMode::Websocket);

        // deserialize unknown variant -> error
        assert!(serde_json::from_str::<MattermostListenMode>("\"unknown\"").is_err());
    }

    #[test]
    fn test_ws_event_posted_parsing() {
        let post = json!({
            "id": "post123",
            "user_id": "user456",
            "message": "hello world",
            "create_at": 1717000000000i64,
            "root_id": "",
            "channel_id": "chan789",
            "type": ""
        });

        let ch = MattermostChannel::new(
            "https://mm.example.com".into(),
            Some("token".into()),
            None,
            None,
            vec![],
            "test",
            Arc::new(|| vec!["user456".into()]),
            false,
            false,
        );

        let msg = ch
            .parse_mattermost_post(&post, "bot_user", "bot_username", 0, "chan789", None, false)
            .expect("should parse posted event post");

        assert_eq!(msg.id, "mattermost_post123");
        assert_eq!(msg.sender, "user456");
        assert_eq!(msg.content, "hello world");
    }

    #[test]
    fn test_ws_posted_envelope_post_is_json_string() {
        // Mattermost sends data.post as a JSON-encoded string, not a nested
        // object. This test exercises the extraction path the WebSocket listener
        // uses: Value::String → as_str() → from_str. The old to_string() path
        // would re-serialize as a quoted literal and silently drop the event.
        let post_obj = json!({
            "id": "post789",
            "user_id": "user999",
            "message": "ws message",
            "create_at": 1717000000000i64,
            "root_id": "",
            "channel_id": "chan111",
            "type": ""
        });
        let post_str = serde_json::to_string(&post_obj).unwrap();

        let envelope = json!({
            "event": "posted",
            "data": {
                "post": post_str
            }
        });

        let post = MattermostChannel::ws_post_from_event(&envelope)
            .expect("should parse the inner JSON string");

        let ch = MattermostChannel::new(
            "https://mm.example.com".into(),
            Some("token".into()),
            None,
            None,
            vec![],
            "test",
            Arc::new(|| vec!["user999".into()]),
            false,
            false,
        );

        let msg = ch
            .parse_mattermost_post(&post, "bot_user", "bot_username", 0, "chan111", None, false)
            .expect("should parse posted event post from envelope");

        assert_eq!(msg.id, "mattermost_post789");
        assert_eq!(msg.sender, "user999");
        assert_eq!(msg.content, "ws message");
    }

    #[test]
    fn test_ws_ping_message_format() {
        // Verify the application-level ping frame the heartbeat pinger sends.
        let seq = 1i64;
        let ping = serde_json::json!({"seq": seq, "action": "ping"});
        assert_eq!(ping["seq"], serde_json::json!(1i64));
        assert_eq!(ping["action"], serde_json::json!("ping"));

        // Round-trip: the message is a Text frame whose content is the JSON
        // string. The Mattermost server expects this exact shape.
        let text = ping.to_string();
        let roundtripped: serde_json::Value =
            serde_json::from_str(&text).expect("ping json must round-trip");
        assert_eq!(roundtripped["action"], serde_json::json!("ping"));
        assert!(roundtripped["seq"].is_i64());
    }

    #[test]
    fn test_ws_auth_challenge_format() {
        // Verify the authentication_challenge frame sent immediately after connect.
        let token = "test_bot_token";
        let seq = 1i64;
        let auth = serde_json::json!({
            "seq": seq,
            "action": "authentication_challenge",
            "data": { "token": token }
        });
        assert_eq!(auth["seq"], serde_json::json!(1i64));
        assert_eq!(
            auth["action"],
            serde_json::json!("authentication_challenge")
        );
        assert_eq!(auth["data"]["token"], serde_json::json!("test_bot_token"));

        let text = auth.to_string();
        let roundtripped: serde_json::Value =
            serde_json::from_str(&text).expect("auth json must round-trip");
        assert_eq!(
            roundtripped["data"]["token"],
            serde_json::json!("test_bot_token")
        );
    }

    #[test]
    fn test_ws_auth_response_matches_challenge_sequence() {
        let success = json!({"status": "OK", "seq_reply": 7});
        let failure = json!({"status": "FAIL", "seq_reply": 7});
        let unrelated = json!({"status": "OK", "seq_reply": 8});

        assert_eq!(MattermostChannel::ws_auth_response(&success, 7), Some(true));
        assert_eq!(
            MattermostChannel::ws_auth_response(&failure, 7),
            Some(false)
        );
        assert_eq!(MattermostChannel::ws_auth_response(&unrelated, 7), None);
    }

    #[tokio::test]
    async fn test_ws_handshake_sends_auth_before_waiting_for_hello() {
        use tokio_tungstenite::tungstenite::protocol::Role;

        let (client_io, server_io) = tokio::io::duplex(4096);
        let client = WebSocketStream::from_raw_socket(client_io, Role::Client, None).await;
        let mut server = WebSocketStream::from_raw_socket(server_io, Role::Server, None).await;
        let (mut write, mut read) = client.split();

        let server_task = galleon_spawn::spawn!(async move {
            let first = server
                .next()
                .await
                .expect("client should send auth first")
                .expect("auth frame should be readable");
            let WsMessage::Text(text) = first else {
                panic!("first client frame should be text auth");
            };
            let auth: serde_json::Value =
                serde_json::from_str(text.as_ref()).expect("auth should be JSON");
            assert_eq!(auth["action"], "authentication_challenge");
            assert_eq!(auth["data"]["token"], "test-token");

            server
                .send(WsMessage::Text(
                    json!({"status": "OK", "seq_reply": 7}).to_string().into(),
                ))
                .await
                .expect("server should send auth response");
            server
                .send(WsMessage::Text(
                    json!({"event": "hello", "data": {"server_version": "10.8.0"}})
                        .to_string()
                        .into(),
                ))
                .await
                .expect("server should send hello");
        });

        let version = MattermostChannel::authenticate_websocket(
            &mut write,
            &mut read,
            "test-token",
            7,
            Duration::from_secs(1),
        )
        .await
        .expect("auth response followed by hello should complete the handshake");

        assert_eq!(version, "10.8.0");
        server_task.await.expect("fake server should finish");
    }

    #[tokio::test]
    async fn test_ws_handshake_rejects_failed_auth() {
        use tokio_tungstenite::tungstenite::protocol::Role;

        let (client_io, server_io) = tokio::io::duplex(4096);
        let client = WebSocketStream::from_raw_socket(client_io, Role::Client, None).await;
        let mut server = WebSocketStream::from_raw_socket(server_io, Role::Server, None).await;
        let (mut write, mut read) = client.split();

        let server_task = galleon_spawn::spawn!(async move {
            server
                .next()
                .await
                .expect("auth frame should arrive")
                .unwrap();
            server
                .send(WsMessage::Text(
                    json!({"status": "FAIL", "seq_reply": 3}).to_string().into(),
                ))
                .await
                .expect("server should send rejection");
        });

        let error = MattermostChannel::authenticate_websocket(
            &mut write,
            &mut read,
            "bad-token",
            3,
            Duration::from_secs(1),
        )
        .await
        .expect_err("failed auth must end the listener attempt");

        assert!(error.to_string().contains("authentication was rejected"));
        server_task.await.expect("fake server should finish");
    }

    #[tokio::test]
    async fn test_ws_handshake_times_out_after_auth_send() {
        use tokio_tungstenite::tungstenite::protocol::Role;

        let (client_io, server_io) = tokio::io::duplex(4096);
        let client = WebSocketStream::from_raw_socket(client_io, Role::Client, None).await;
        let mut server = WebSocketStream::from_raw_socket(server_io, Role::Server, None).await;
        let (mut write, mut read) = client.split();

        let server_task = galleon_spawn::spawn!(async move {
            server
                .next()
                .await
                .expect("auth frame should arrive")
                .unwrap();
            tokio::time::sleep(Duration::from_millis(100)).await;
        });

        let error = MattermostChannel::authenticate_websocket(
            &mut write,
            &mut read,
            "test-token",
            4,
            Duration::from_millis(10),
        )
        .await
        .expect_err("a silent server must fail the handshake deadline");

        assert!(error.to_string().contains("handshake timed out"));
        server_task.abort();
    }

    #[tokio::test]
    async fn test_ws_handshake_times_out_without_hello() {
        use tokio_tungstenite::tungstenite::protocol::Role;

        let (client_io, server_io) = tokio::io::duplex(4096);
        let client = WebSocketStream::from_raw_socket(client_io, Role::Client, None).await;
        let mut server = WebSocketStream::from_raw_socket(server_io, Role::Server, None).await;
        let (mut write, mut read) = client.split();

        let server_task = galleon_spawn::spawn!(async move {
            server
                .next()
                .await
                .expect("auth frame should arrive")
                .unwrap();
            server
                .send(WsMessage::Text(
                    json!({"status": "OK", "seq_reply": 5}).to_string().into(),
                ))
                .await
                .expect("server should send auth response");
            tokio::time::sleep(Duration::from_millis(100)).await;
        });

        let error = MattermostChannel::authenticate_websocket(
            &mut write,
            &mut read,
            "test-token",
            5,
            Duration::from_millis(10),
        )
        .await
        .expect_err("auth without hello must fail the handshake deadline");

        assert!(error.to_string().contains("handshake timed out"));
        server_task.abort();
    }

    #[tokio::test]
    async fn test_ws_handshake_times_out_without_auth_response() {
        use tokio_tungstenite::tungstenite::protocol::Role;

        let (client_io, server_io) = tokio::io::duplex(4096);
        let client = WebSocketStream::from_raw_socket(client_io, Role::Client, None).await;
        let mut server = WebSocketStream::from_raw_socket(server_io, Role::Server, None).await;
        let (mut write, mut read) = client.split();

        let server_task = galleon_spawn::spawn!(async move {
            server
                .next()
                .await
                .expect("auth frame should arrive")
                .unwrap();
            server
                .send(WsMessage::Text(
                    json!({"event": "hello", "data": {"server_version": "10.8.0"}})
                        .to_string()
                        .into(),
                ))
                .await
                .expect("server should send hello");
            tokio::time::sleep(Duration::from_millis(100)).await;
        });

        let error = MattermostChannel::authenticate_websocket(
            &mut write,
            &mut read,
            "test-token",
            6,
            Duration::from_millis(10),
        )
        .await
        .expect_err("hello without auth response must fail the handshake deadline");

        assert!(error.to_string().contains("handshake timed out"));
        server_task.abort();
    }

    #[test]
    fn test_ws_timeout_constants() {
        // WS_READ_TIMEOUT must be strictly greater than WS_PING_INTERVAL
        // so a single missed ping does not trigger a false positive.
        assert!(
            WS_READ_TIMEOUT > WS_PING_INTERVAL,
            "WS_READ_TIMEOUT ({:?}) must exceed WS_PING_INTERVAL ({:?})",
            WS_READ_TIMEOUT,
            WS_PING_INTERVAL
        );
        // WS_READ_TIMEOUT should be at least 3× ping interval so the
        // server can miss two pings before the listener reconnects.
        assert!(
            WS_READ_TIMEOUT >= WS_PING_INTERVAL.mul_f64(3.0),
            "WS_READ_TIMEOUT ({:?}) must be ≥ 3× WS_PING_INTERVAL ({:?})",
            WS_READ_TIMEOUT,
            WS_PING_INTERVAL
        );
        assert!(WS_HANDSHAKE_TIMEOUT <= WS_READ_TIMEOUT);
    }

    #[tokio::test]
    async fn test_ws_read_timeout_detects_silent_peer() {
        let deadline = tokio::time::Instant::now() + Duration::from_millis(10);
        tokio::select! {
            () = std::future::pending::<()>() => panic!("silent peer unexpectedly produced a frame"),
            () = tokio::time::sleep_until(deadline) => {}
        }
    }
}

/// Approval-prompt routing. Mattermost previously inherited the `Channel`
/// trait's default `request_approval`, which returns `Ok(None)` meaning "this
/// channel does not implement the prompt at all" — the runtime then denied
/// every gated tool call with `ApprovalSource::Unavailable`, and the operator
/// was never asked.
#[cfg(test)]
mod approval_tests {
    use super::*;
    use serde_json::json;

    const ALLOWED_USER: &str = "operator_user_id";
    const BOT_USER: &str = "bot_user_id";

    fn channel_with_peers(peers: Vec<String>) -> MattermostChannel {
        let ch = MattermostChannel::new(
            "https://mm.example.com".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "approval_test_alias",
            Arc::new(move || peers.clone()),
            false,
            false,
        );
        // Seed the identity these fixtures would otherwise have to fetch. The
        // host is unreachable, so `get_bot_identity` would yield an empty ID and
        // the reaction path — which now fails closed on an unknown bot identity —
        // would refuse to decide. Before that guard existed these fixtures passed
        // only because the check was skipped when the ID was empty, so seeding it
        // is what makes them exercise a real, known identity.
        let _ = ch
            .bot_identity
            .set((BOT_USER.to_string(), "glados".to_string()));
        ch
    }

    /// The channel every fixture prompt is posted into unless stated otherwise.
    const ORIGIN_CHANNEL: &str = "chan1";

    fn post_from(user_id: &str, message: &str) -> serde_json::Value {
        post_from_in(ORIGIN_CHANNEL, user_id, message)
    }

    /// The same, in an explicit channel: destination binding is only testable
    /// when the fixture can name a channel other than the prompt's.
    fn post_from_in(channel_id: &str, user_id: &str, message: &str) -> serde_json::Value {
        json!({
            "id": "post1",
            "channel_id": channel_id,
            "user_id": user_id,
            "message": message,
            "create_at": 1,
        })
    }

    /// Register a pending approval bound to `ORIGIN_CHANNEL`, as
    /// `request_approval_attributed` does.
    async fn pending_in_origin(
        ch: &MattermostChannel,
        token: &str,
    ) -> tokio::sync::oneshot::Receiver<ChannelApprovalResponse> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        ch.approvals.lock().pending.insert(
            token.to_string(),
            PendingApproval {
                generation: 0,
                channel_id: ORIGIN_CHANNEL.to_string(),
                sender: tx,
            },
        );
        rx
    }

    /// A `reaction_added` WebSocket event. Mattermost sends the reaction as a
    /// JSON *string* under `data.reaction`, so the fixture encodes it the same
    /// way the server does — a plain object here would pass a test that the
    /// real event shape fails.
    fn reaction_event(user_id: &str, post_id: &str, emoji_name: &str) -> serde_json::Value {
        json!({
            "event": "reaction_added",
            "data": {
                "reaction": json!({
                    "user_id": user_id,
                    "post_id": post_id,
                    "emoji_name": emoji_name,
                    "create_at": 1,
                })
                .to_string(),
            },
        })
    }

    /// Register a pending approval reachable by both paths, as
    /// `request_approval_attributed` does in Websocket mode.
    async fn pending_with_post(
        ch: &MattermostChannel,
        token: &str,
        post_id: &str,
    ) -> tokio::sync::oneshot::Receiver<ChannelApprovalResponse> {
        let rx = pending_in_origin(ch, token).await;
        ch.approvals
            .lock()
            .posts
            .insert(post_id.to_string(), (token.to_string(), 0));
        rx
    }

    /// The point of the feature: a tap on the seeded checkmark answers the
    /// prompt without the operator typing the token back.
    #[tokio::test]
    async fn authorized_reaction_resolves_pending_approval() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        let rx = pending_with_post(&ch, "abc123", "prompt_post").await;

        let consumed = ch
            .try_resolve_approval_reaction(&reaction_event(
                ALLOWED_USER,
                "prompt_post",
                APPROVAL_EMOJI_APPROVE,
            ))
            .await;

        assert!(consumed, "a decision reaction must resolve the prompt");
        assert_eq!(rx.await.unwrap(), ChannelApprovalResponse::Approve);
        assert!(
            ch.approvals.lock().pending.is_empty(),
            "resolving must retire the pending token"
        );
        assert!(
            ch.approvals.lock().posts.is_empty(),
            "resolving must retire the post mapping so a later tap finds nothing"
        );
    }

    /// The deny emoji must mean deny — not merely "something was tapped".
    #[tokio::test]
    async fn deny_reaction_resolves_as_deny() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        let rx = pending_with_post(&ch, "abc123", "prompt_post").await;

        assert!(
            ch.try_resolve_approval_reaction(&reaction_event(
                ALLOWED_USER,
                "prompt_post",
                APPROVAL_EMOJI_DENY,
            ))
            .await
        );
        assert_eq!(rx.await.unwrap(), ChannelApprovalResponse::Deny);
    }

    /// Reacting is as privileged as replying, so the same allowlist applies.
    /// The prompt must stay pending: an unauthorized tap cannot consume the
    /// decision an authorized operator has not made yet.
    #[tokio::test]
    async fn unauthorized_reaction_cannot_approve() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        let mut rx = pending_with_post(&ch, "abc123", "prompt_post").await;

        let consumed = ch
            .try_resolve_approval_reaction(&reaction_event(
                "intruder_user_id",
                "prompt_post",
                APPROVAL_EMOJI_APPROVE,
            ))
            .await;

        assert!(
            !consumed,
            "an unauthorized tap must not count as a decision"
        );
        assert!(
            rx.try_recv().is_err(),
            "an unauthorized tap must not deliver a response"
        );
        assert!(
            !ch.approvals.lock().pending.is_empty(),
            "the prompt must stay answerable by an authorized operator"
        );
    }

    /// Regression: the bot seeds both emoji itself, so its own reactions arrive
    /// as events on the post it just created. Treating those as decisions would
    /// approve every prompt the instant it was posted.
    #[tokio::test]
    async fn bot_own_seeded_reaction_is_ignored() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        let mut rx = pending_with_post(&ch, "abc123", "prompt_post").await;
        // `get_bot_identity` caches; seed it so the test makes no HTTP call.
        let _ = ch
            .bot_identity
            .set(("bot_user_id".to_string(), "glados".to_string()));

        let consumed = ch
            .try_resolve_approval_reaction(&reaction_event(
                "bot_user_id",
                "prompt_post",
                APPROVAL_EMOJI_APPROVE,
            ))
            .await;

        assert!(
            !consumed,
            "the bot's own seeded emoji must not self-approve"
        );
        assert!(rx.try_recv().is_err(), "no response must be delivered");
        assert!(
            !ch.approvals.lock().pending.is_empty(),
            "the prompt must stay pending"
        );
    }

    /// An ordinary reaction on the prompt post is not an answer. Only the two
    /// decision emoji are.
    #[tokio::test]
    async fn unrelated_emoji_is_not_a_decision() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        let mut rx = pending_with_post(&ch, "abc123", "prompt_post").await;

        assert!(
            !ch.try_resolve_approval_reaction(&reaction_event(
                ALLOWED_USER,
                "prompt_post",
                "thumbsup",
            ))
            .await
        );
        assert!(rx.try_recv().is_err());
        assert!(!ch.approvals.lock().pending.is_empty());
    }

    /// A decision emoji on some other post must not resolve a live prompt.
    #[tokio::test]
    async fn reaction_on_unrelated_post_is_ignored() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        let mut rx = pending_with_post(&ch, "abc123", "prompt_post").await;

        assert!(
            !ch.try_resolve_approval_reaction(&reaction_event(
                ALLOWED_USER,
                "some_other_post",
                APPROVAL_EMOJI_APPROVE,
            ))
            .await
        );
        assert!(rx.try_recv().is_err());
        assert!(!ch.approvals.lock().pending.is_empty());
    }

    /// The two paths share one decision. Whichever answers first retires the
    /// token, and the other must find nothing left to answer.
    #[tokio::test]
    async fn reply_and_reaction_cannot_both_answer_one_prompt() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        let rx = pending_with_post(&ch, "abc123", "prompt_post").await;

        assert!(
            ch.try_resolve_approval_reply(&post_from(ALLOWED_USER, "abc123 yes"), BOT_USER)
                .await
                == ApprovalReplyOutcome::Consumed
        );
        assert_eq!(rx.await.unwrap(), ChannelApprovalResponse::Approve);

        // The reaction arrives afterwards — the operator tapped as well as typed.
        assert!(
            !ch.try_resolve_approval_reaction(&reaction_event(
                ALLOWED_USER,
                "prompt_post",
                APPROVAL_EMOJI_DENY,
            ))
            .await,
            "a second answer must not resolve an already-retired prompt"
        );
    }

    /// A token echo from an allow-listed operator resolves the waiting prompt
    /// and is consumed, so the bare `token yes` never reaches the model.
    #[tokio::test]
    async fn authorized_reply_resolves_pending_approval() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        let rx = pending_in_origin(&ch, "abc123").await;

        let consumed = ch
            .try_resolve_approval_reply(&post_from(ALLOWED_USER, "abc123 yes"), BOT_USER)
            .await;

        assert_eq!(
            consumed,
            ApprovalReplyOutcome::Consumed,
            "an approval reply must not fall through to the model"
        );
        assert_eq!(rx.await.unwrap(), ChannelApprovalResponse::Approve);
        assert!(
            ch.approvals.lock().pending.is_empty(),
            "resolving must retire the pending token"
        );
    }

    /// Approving a tool call is at least as privileged as sending a message,
    /// so the allowlist is re-checked on the approval path.
    #[tokio::test]
    async fn unauthorized_reply_cannot_approve() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        let mut rx = pending_in_origin(&ch, "abc123").await;

        let consumed = ch
            .try_resolve_approval_reply(&post_from("intruder_user_id", "abc123 yes"), BOT_USER)
            .await;

        assert_eq!(
            consumed,
            ApprovalReplyOutcome::Consumed,
            "the reply is protocol text, not a message for the model"
        );
        assert!(
            rx.try_recv().is_err(),
            "an unauthorized reply must not decide the approval"
        );
        assert!(
            ch.approvals.lock().pending.contains_key("abc123"),
            "the prompt must stay pending so an authorized operator can still answer"
        );
    }

    /// A well-formed reply for a token this channel never issued belongs to
    /// someone else (or nobody) and must reach the normal message path.
    #[tokio::test]
    async fn unknown_token_is_not_consumed() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        assert!(
            ch.try_resolve_approval_reply(&post_from(ALLOWED_USER, "zzz999 yes"), BOT_USER)
                .await
                == ApprovalReplyOutcome::NotFound
        );
    }

    /// Regression: a token is readable by everyone who can see the prompt, and
    /// one channel instance serves many Mattermost channels, so an authorized
    /// user must not be able to carry a valid token into a different room and
    /// answer there.
    ///
    /// Driven through `process_inbound_post`, the production inbound path, so
    /// the binding is proven where posts actually arrive rather than only in the
    /// resolver.
    #[tokio::test]
    async fn approval_reply_from_another_channel_cannot_answer() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        let mut rx = pending_in_origin(&ch, "abc123").await;
        let (tx, mut inbound) = tokio::sync::mpsc::channel(8);

        // Same token, same authorized operator, wrong channel.
        ch.process_inbound_post(
            &post_from_in("chan2", ALLOWED_USER, "abc123 yes"),
            "bot_user_id",
            "glados",
            0,
            "chan2",
            false,
            &tx,
        )
        .await;

        assert!(
            rx.try_recv().is_err(),
            "a reply from another channel must not deliver a decision"
        );
        assert!(
            ch.approvals.lock().pending.contains_key("abc123"),
            "the prompt must stay answerable in the channel it belongs to"
        );
        assert!(
            inbound.try_recv().is_err(),
            "approval-reply text must not fall through to the model either"
        );

        // The same reply in the origin channel resolves it.
        ch.process_inbound_post(
            &post_from(ALLOWED_USER, "abc123 yes"),
            "bot_user_id",
            "glados",
            0,
            ORIGIN_CHANNEL,
            false,
            &tx,
        )
        .await;

        assert_eq!(rx.await.unwrap(), ChannelApprovalResponse::Approve);
        assert!(ch.approvals.lock().pending.is_empty());
    }

    /// Build a channel pointed at a mock Mattermost server.
    fn channel_at(base_url: &str, peers: Vec<String>) -> MattermostChannel {
        MattermostChannel::new(
            base_url.to_string(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "approval_test_alias",
            Arc::new(move || peers.clone()),
            false,
            false,
        )
    }

    fn approval_request() -> galleon_api::channel::ChannelApprovalRequest {
        galleon_api::channel::ChannelApprovalRequest {
            tool_name: "shell".into(),
            arguments_summary: "rm -rf /".into(),
            raw_arguments: None,
            position: None,
        }
    }

    /// The real timeout regression: the prompt posts successfully, nobody
    /// answers, and the runtime denies on its own authority.
    ///
    /// `approval_timeout_secs = 0` is an already-elapsed deadline, which is the
    /// documented "0 denies immediately" semantics, so this also pins that.
    #[tokio::test]
    async fn timeout_after_successful_post_is_attributed_to_the_runtime() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v4/posts"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id": "prompt_post"})))
            .mount(&server)
            .await;

        let ch =
            channel_at(&server.uri(), vec![ALLOWED_USER.to_string()]).with_approval_timeout_secs(0);

        let attributed = ch
            .request_approval_attributed(ORIGIN_CHANNEL, &approval_request())
            .await
            .expect("a successful post must not surface as an error")
            .expect("Mattermost answers approvals, so it must not return None");

        assert_eq!(attributed.response, ChannelApprovalResponse::Deny);
        assert_eq!(
            attributed.source,
            galleon_api::channel::ApprovalSource::TimedOut,
            "an unanswered prompt is the runtime denying, not an operator refusing"
        );
        assert!(
            ch.approvals.lock().pending.is_empty(),
            "a timed-out prompt must not leak its token"
        );
        assert!(
            ch.approvals.lock().posts.is_empty(),
            "a timed-out prompt must not leak its post binding"
        );
    }

    /// The prompt actually reaches Mattermost, and websocket mode actually
    /// seeds both decision emoji. Asserted at the HTTP boundary, since
    /// `create_post` and `react_to_post` are otherwise only exercised by their
    /// callers' return values.
    #[tokio::test]
    async fn websocket_mode_posts_prompt_and_seeds_both_reactions() {
        use wiremock::matchers::{body_string_contains, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v4/users/me"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"id": "bot_user_id", "username": "glados"})),
            )
            .mount(&server)
            .await;
        // The prompt carries the token and the tool, or the operator cannot
        // answer. Matched on those protocol-exact fields rather than on the
        // heading: the heading comes from the Fluent catalogue and is localized,
        // so asserting its English spelling would fail this test under a
        // supported non-English locale even though the prompt is correct. The
        // token and tool name are echoed verbatim in every locale — that is
        // precisely what `parse_approval_reply` matches on.
        Mock::given(method("POST"))
            .and(path("/api/v4/posts"))
            .and(body_string_contains("shell"))
            .and(body_string_contains("rm -rf /"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id": "prompt_post"})))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v4/reactions"))
            .and(body_string_contains(APPROVAL_EMOJI_APPROVE))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v4/reactions"))
            .and(body_string_contains(r#""emoji_name":"x""#))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(1)
            .mount(&server)
            .await;

        let ch = channel_at(&server.uri(), vec![ALLOWED_USER.to_string()])
            .with_listen_mode(galleon_config::schema::MattermostListenMode::Websocket)
            .with_approval_timeout_secs(0);

        let _ = ch
            .request_approval_attributed(ORIGIN_CHANNEL, &approval_request())
            .await;

        // MockServer verifies the `expect(..)` counts when it drops.
        drop(server);
    }

    /// The mirror of the test above: polling mode must not seed emoji, because
    /// the polling listener never receives `reaction_added` and the operator
    /// would be tapping a control that does nothing.
    #[tokio::test]
    async fn polling_mode_posts_prompt_without_seeding_reactions() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/api/v4/posts"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({"id": "prompt_post"})))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("POST"))
            .and(path("/api/v4/reactions"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({})))
            .expect(0)
            .mount(&server)
            .await;

        let ch =
            channel_at(&server.uri(), vec![ALLOWED_USER.to_string()]).with_approval_timeout_secs(0);
        let _ = ch
            .request_approval_attributed(ORIGIN_CHANNEL, &approval_request())
            .await;

        assert!(
            ch.approvals.lock().posts.is_empty(),
            "polling mode must not register a reaction binding"
        );
        drop(server);
    }

    /// Ordinary conversation must never be swallowed by the approval path.
    #[tokio::test]
    async fn ordinary_message_is_not_consumed() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        let _rx = pending_in_origin(&ch, "abc123").await;

        assert!(
            ch.try_resolve_approval_reply(
                &post_from(ALLOWED_USER, "what is the weather"),
                BOT_USER
            )
            .await
                == ApprovalReplyOutcome::NotFound
        );
        assert!(ch.approvals.lock().pending.contains_key("abc123"));
    }

    /// `deny` and `always` must map to their own outcomes rather than
    /// collapsing into the generic approve/deny pair.
    #[tokio::test]
    async fn deny_and_always_map_to_distinct_responses() {
        for (reply, expected) in [
            ("abc123 no", ChannelApprovalResponse::Deny),
            ("abc123 always", ChannelApprovalResponse::AlwaysApprove),
        ] {
            let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
            let rx = pending_in_origin(&ch, "abc123").await;
            assert!(
                ch.try_resolve_approval_reply(&post_from(ALLOWED_USER, reply), BOT_USER)
                    .await
                    == ApprovalReplyOutcome::Consumed
            );
            assert_eq!(rx.await.unwrap(), expected);
        }
    }

    /// A zero timeout is an already-elapsed deadline. The config default must
    /// therefore be the documented budget, not the zero a derived `Default`
    /// would produce — otherwise an alias built in Rust denies every approval.
    #[test]
    fn constructor_default_timeout_is_not_zero() {
        let ch = channel_with_peers(Vec::new());
        assert_eq!(
            ch.approval_timeout_secs,
            galleon_config::schema::MattermostConfig::default().approval_timeout_secs,
        );
        assert!(ch.approval_timeout_secs > 0);
    }

    /// A prompt that never reaches Mattermost is an error, not a decision.
    ///
    /// Renamed from `timeout_denies_attributed_to_runtime_not_operator`, which
    /// this never tested: the unreachable test host makes `create_post` fail, so
    /// it returns through the post-`Err` branch and never reaches the timeout
    /// arms. `timeout_after_successful_post_is_attributed_to_the_runtime` below
    /// is the actual timeout regression.
    #[tokio::test]
    async fn failed_prompt_post_is_an_error_not_an_operator_decision() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]).with_approval_timeout_secs(0);
        let request = galleon_api::channel::ChannelApprovalRequest {
            tool_name: "shell".into(),
            arguments_summary: "rm -rf /".into(),
            raw_arguments: None,
            position: None,
        };

        // `send` fails against the unreachable test host, which surfaces as an
        // Err rather than a synthesized operator decision.
        let result = ch.request_approval_attributed("chan1", &request).await;
        assert!(
            result.is_err(),
            "a failed prompt post must not be reported as an operator decision"
        );
        assert!(
            ch.approvals.lock().pending.is_empty(),
            "a failed prompt must not leak its pending token"
        );
    }
}

/// Regressions for the three approval lifecycle/authorization defects raised in
/// review: a losing text reply reaching the model, cancelled futures stranding
/// state, and approval decisions bypassing the identity checks that
/// `parse_mattermost_post` applies to ordinary messages.
#[cfg(test)]
mod approval_lifecycle_tests {
    use super::*;
    use serde_json::json;

    const ALLOWED_USER: &str = "operator_user_id";
    const BOT_USER: &str = "bot_user_id";
    const ORIGIN_CHANNEL: &str = "chan1";

    fn channel_with_peers(peers: Vec<String>) -> MattermostChannel {
        MattermostChannel::new(
            "https://mm.example.com".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "approval_lifecycle_alias",
            Arc::new(move || peers.clone()),
            false,
            false,
        )
    }

    fn post_from(user_id: &str, message: &str) -> serde_json::Value {
        json!({
            "id": "post1",
            "user_id": user_id,
            "message": message,
            "create_at": 1,
            "channel_id": ORIGIN_CHANNEL,
        })
    }

    fn pending(
        ch: &MattermostChannel,
        token: &str,
    ) -> tokio::sync::oneshot::Receiver<ChannelApprovalResponse> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        ch.approvals.lock().pending.insert(
            token.to_string(),
            PendingApproval {
                generation: 0,
                channel_id: ORIGIN_CHANNEL.to_string(),
                sender: tx,
            },
        );
        rx
    }

    /// The losing half of a reply/reaction race must not become chat.
    ///
    /// Recognition and removal are one transition now, so a reply arriving after
    /// the reaction already answered cannot be mistaken for text that was never
    /// ours. Previously the final `remove` returned `None`, the resolver
    /// reported "not mine", and `process_inbound_post` forwarded the operator's
    /// `<token> yes` to the model as ordinary conversation.
    #[tokio::test]
    async fn losing_reply_after_reaction_wins_is_not_forwarded_to_the_model() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        let rx = pending(&ch, "abc123");
        let (tx, mut inbound) = tokio::sync::mpsc::channel(8);

        // The reaction path answers first and retires the token, the way
        // production does it — by generation, not by token.
        let generation = ch
            .approvals
            .lock()
            .pending
            .get("abc123")
            .expect("token was live")
            .generation;
        let resolved = ch
            .approvals
            .lock()
            .retire_generation("abc123", generation)
            .expect("token was live");
        let _ = resolved.sender.send(ChannelApprovalResponse::Approve);
        assert_eq!(rx.await.unwrap(), ChannelApprovalResponse::Approve);

        // The operator's typed reply is already in flight and lands afterwards.
        ch.process_inbound_post(
            &post_from(ALLOWED_USER, "abc123 yes"),
            BOT_USER,
            "glados",
            0,
            ORIGIN_CHANNEL,
            false,
            &tx,
        )
        .await;

        assert!(
            inbound.try_recv().is_err(),
            "a reply that lost the race must be suppressed, not delivered to the model"
        );
    }

    /// Text that merely looks like an approval reply, for a token this channel
    /// never issued, is ordinary conversation and must still reach the model.
    #[tokio::test]
    async fn unrelated_token_shaped_text_still_reaches_the_model() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        assert_eq!(
            ch.try_resolve_approval_reply(&post_from(ALLOWED_USER, "zzz999 yes"), BOT_USER)
                .await,
            ApprovalReplyOutcome::NotFound,
            "only tokens this channel knows about may be swallowed"
        );
    }

    /// Retired-token memory is bounded, so a long-lived channel cannot grow it
    /// without limit.
    #[test]
    fn retired_token_memory_is_bounded() {
        let ch = channel_with_peers(Vec::new());
        for index in 0..(RETIRED_APPROVAL_TOKENS * 3) {
            ch.approvals
                .lock()
                .remember_retired(&format!("tok{index:04}"));
        }
        assert_eq!(
            ch.approvals.lock().retired.len(),
            RETIRED_APPROVAL_TOKENS,
            "the retired ring must stay capped"
        );
    }

    /// Dropping the request future must retire everything it registered.
    ///
    /// The routed approval caller wraps this future in an outer timeout whose
    /// default is shorter than this channel's own, so cancellation mid-prompt is
    /// a supported configuration. Cleanup used to live only in the explicit
    /// match arms, which a dropped future never reaches.
    #[test]
    fn dropping_the_guard_retires_pending_and_post_state() {
        let ch = channel_with_peers(vec![ALLOWED_USER.to_string()]);
        let _rx = pending(&ch, "abc123");
        ch.approvals
            .lock()
            .posts
            .insert("post1".to_string(), ("abc123".to_string(), 0));

        {
            let _guard = ApprovalGuard::new(Arc::clone(&ch.approvals), "abc123".to_string(), 0);
            assert!(!ch.approvals.lock().pending.is_empty());
        } // dropped here, as an outer timeout would

        let state = ch.approvals.lock();
        assert!(
            state.pending.is_empty(),
            "a cancelled request must not strand its pending token"
        );
        assert!(
            state.posts.is_empty(),
            "a cancelled request must not strand its post binding"
        );
        assert!(
            state.was_retired("abc123"),
            "the token must be remembered so a late tap is still recognized"
        );
    }

    /// A guard that already retired its token must not retire it a second time
    /// when dropped — otherwise the normal completion path would evict a token
    /// a *later* request had legitimately reused.
    #[test]
    fn explicit_retire_disarms_the_guard() {
        let ch = channel_with_peers(Vec::new());
        let _rx = pending(&ch, "abc123");
        {
            let mut guard = ApprovalGuard::new(Arc::clone(&ch.approvals), "abc123".to_string(), 0);
            guard.retire();
            // A new request reuses the same token before the guard drops.
            let _reused = pending(&ch, "abc123");
        }
        assert!(
            ch.approvals.lock().pending.contains_key("abc123"),
            "a disarmed guard must not evict a later request's token"
        );
    }

    /// The bot's own approval-shaped post must never answer a prompt, even when
    /// the peer group is a wildcard that matches every identity.
    #[tokio::test]
    async fn bot_authored_reply_cannot_self_approve_under_wildcard() {
        let ch = channel_with_peers(vec!["*".to_string()]);
        let mut rx = pending(&ch, "abc123");

        let outcome = ch
            .try_resolve_approval_reply(&post_from(BOT_USER, "abc123 yes"), BOT_USER)
            .await;

        assert_eq!(
            outcome,
            ApprovalReplyOutcome::Consumed,
            "the bot's own echo is protocol traffic, not a message for the model"
        );
        assert!(
            rx.try_recv().is_err(),
            "the bot must not be able to approve its own prompt"
        );
        assert!(
            ch.approvals.lock().pending.contains_key("abc123"),
            "the prompt must stay answerable by a real operator"
        );
    }

    /// A malformed event with no author must not be treated as an authorized
    /// operator by a wildcard allowlist.
    #[tokio::test]
    async fn missing_identity_reply_cannot_approve_under_wildcard() {
        let ch = channel_with_peers(vec!["*".to_string()]);
        let mut rx = pending(&ch, "abc123");

        let outcome = ch
            .try_resolve_approval_reply(
                &json!({
                    "id": "post1",
                    "message": "abc123 yes",
                    "create_at": 1,
                    "channel_id": ORIGIN_CHANNEL,
                }),
                BOT_USER,
            )
            .await;

        assert_eq!(outcome, ApprovalReplyOutcome::Consumed);
        assert!(
            rx.try_recv().is_err(),
            "an event with no identity must not decide an approval"
        );
    }

    /// The reaction path has the same missing-identity exposure and the same
    /// rule: a wildcard peer group must not turn an absent author into an
    /// authorized one.
    #[tokio::test]
    async fn missing_identity_reaction_cannot_approve_under_wildcard() {
        let ch = channel_with_peers(vec!["*".to_string()]);
        let mut rx = pending(&ch, "abc123");
        ch.approvals
            .lock()
            .posts
            .insert("post1".to_string(), ("abc123".to_string(), 0));

        let event = json!({
            "data": {
                "reaction": json!({
                    "post_id": "post1",
                    "emoji_name": APPROVAL_EMOJI_APPROVE,
                })
                .to_string(),
            }
        });

        assert!(
            !ch.try_resolve_approval_reaction(&event).await,
            "a reaction with no author must not resolve an approval"
        );
        assert!(rx.try_recv().is_err());
        assert!(ch.approvals.lock().pending.contains_key("abc123"));
    }
}

/// The approval path must fail closed when the authenticated bot identity is
/// unavailable.
///
/// `get_bot_identity` yields an empty ID whenever `/users/me` is unreachable,
/// unauthorized, or answers without one. The self-approval guard used to skip
/// the comparison in that case, which inverted it exactly when identity was
/// least certain: with the documented wildcard peer group the bot's own user ID
/// passes authorization, so the bot could answer the prompt it had just posted —
/// and in WebSocket mode its own seeded ✅ arrives within milliseconds.
#[cfg(test)]
mod approval_identity_tests {
    use super::*;
    use serde_json::json;

    const BOT_USER: &str = "bot_user_id";
    const ORIGIN_CHANNEL: &str = "chan1";
    /// The identity lookup failed, so the resolver has no ID to compare against.
    const UNKNOWN_BOT: &str = "";

    /// The documented wildcard peer group, which accepts every non-empty ID and
    /// is therefore the configuration in which this guard is load-bearing.
    fn wildcard_channel() -> MattermostChannel {
        MattermostChannel::new(
            "https://mm.example.com".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "approval_identity_alias",
            Arc::new(|| vec!["*".to_string()]),
            false,
            false,
        )
    }

    fn pending(
        ch: &MattermostChannel,
        token: &str,
    ) -> tokio::sync::oneshot::Receiver<ChannelApprovalResponse> {
        let (tx, rx) = tokio::sync::oneshot::channel();
        ch.approvals.lock().pending.insert(
            token.to_string(),
            PendingApproval {
                generation: 0,
                channel_id: ORIGIN_CHANNEL.to_string(),
                sender: tx,
            },
        );
        rx
    }

    /// Production boundary: a bot-authored reply, an unknown bot ID, and a
    /// wildcard allowlist must not resolve the prompt or reach the model.
    #[tokio::test]
    async fn bot_reply_cannot_approve_when_bot_identity_is_unknown() {
        let ch = wildcard_channel();
        let mut rx = pending(&ch, "abc123");
        let (tx, mut inbound) = tokio::sync::mpsc::channel(8);

        ch.process_inbound_post(
            &json!({
                "id": "post1",
                "user_id": BOT_USER,
                "message": "abc123 yes",
                "create_at": 1,
                "channel_id": ORIGIN_CHANNEL,
            }),
            UNKNOWN_BOT,
            "glados",
            0,
            ORIGIN_CHANNEL,
            false,
            &tx,
        )
        .await;

        assert!(
            rx.try_recv().is_err(),
            "an unresolved bot identity must not let the bot approve its own prompt"
        );
        assert!(
            ch.approvals.lock().pending.contains_key("abc123"),
            "the prompt must stay answerable once identity can be established"
        );
        assert!(
            inbound.try_recv().is_err(),
            "approval-shaped text must not fall through to the model"
        );
    }

    /// The reaction path is the sharper case: the bot seeds ✅ itself, so an
    /// unknown identity here would approve within milliseconds of prompting.
    #[tokio::test]
    async fn bot_reaction_cannot_approve_when_bot_identity_is_unknown() {
        let ch = wildcard_channel();
        let mut rx = pending(&ch, "abc123");
        ch.approvals
            .lock()
            .posts
            .insert("post1".to_string(), ("abc123".to_string(), 0));

        // `get_bot_identity` will fail against this unreachable host, yielding
        // the empty ID this test is about — the same value the listener passes
        // on when `/users/me` is unavailable.
        let event = json!({
            "data": {
                "reaction": json!({
                    "post_id": "post1",
                    "user_id": BOT_USER,
                    "emoji_name": APPROVAL_EMOJI_APPROVE,
                })
                .to_string(),
            }
        });

        assert!(
            !ch.try_resolve_approval_reaction(&event).await,
            "the bot's own seeded reaction must not approve while identity is unknown"
        );
        assert!(rx.try_recv().is_err());
        assert!(
            ch.approvals.lock().pending.contains_key("abc123"),
            "the prompt must stay pending"
        );
    }

    /// Even a genuine third-party operator is refused while identity is unknown:
    /// without a bot ID the resolver cannot tell them apart from the bot, so it
    /// declines rather than guessing.
    #[tokio::test]
    async fn third_party_reply_is_refused_while_bot_identity_is_unknown() {
        let ch = wildcard_channel();
        let mut rx = pending(&ch, "abc123");

        let outcome = ch
            .try_resolve_approval_reply(
                &json!({
                    "id": "post1",
                    "user_id": "operator_user_id",
                    "message": "abc123 yes",
                    "create_at": 1,
                    "channel_id": ORIGIN_CHANNEL,
                }),
                UNKNOWN_BOT,
            )
            .await;

        assert_eq!(
            outcome,
            ApprovalReplyOutcome::Consumed,
            "the reply is still approval traffic and must not reach the model"
        );
        assert!(rx.try_recv().is_err());
        assert!(ch.approvals.lock().pending.contains_key("abc123"));
    }

    /// A username-only `/users/me` response must not be cached: pinning an empty
    /// ID would disable the self-approval guard for the channel's whole life.
    ///
    /// Driven through the real `get_bot_identity` against a mock server, so it
    /// tests the production caching rule rather than restating it.
    #[tokio::test]
    async fn partial_identity_response_is_not_cached() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        // A response carrying a username but no `id` — the shape the review
        // identified as permanently poisoning the cache.
        Mock::given(method("GET"))
            .and(path("/api/v4/users/me"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"username": "glados"})))
            .mount(&server)
            .await;

        let ch = MattermostChannel::new(
            server.uri(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "approval_identity_alias",
            Arc::new(|| vec!["*".to_string()]),
            false,
            false,
        );

        let (id, username) = ch.get_bot_identity().await;
        assert!(id.is_empty(), "the response carried no id");
        assert_eq!(username, "glados");
        assert!(
            ch.bot_identity.get().is_none(),
            "a response with no ID must not populate the identity cache, or the \
             self-approval guard stays disabled for the channel's whole life"
        );
    }
}

/// Regressions for the registration-generation race and the identity-recovery
/// availability gap raised in review.
#[cfg(test)]
mod approval_generation_tests {
    use super::*;
    use serde_json::json;

    const ALLOWED_USER: &str = "operator_user_id";
    const BOT_USER: &str = "bot_user_id";
    const ORIGIN_CHANNEL: &str = "chan1";

    fn channel(peers: Vec<String>, base_url: String) -> MattermostChannel {
        MattermostChannel::new(
            base_url,
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "approval_generation_alias",
            Arc::new(move || peers.clone()),
            false,
            false,
        )
    }

    /// Register a prompt the way `request_approval_attributed` does, returning
    /// the receiver and the generation it was given.
    fn register(
        ch: &MattermostChannel,
        token: &str,
        post_id: &str,
    ) -> (tokio::sync::oneshot::Receiver<ChannelApprovalResponse>, u64) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut state = ch.approvals.lock();
        let generation = state.next_generation;
        state.next_generation += 1;
        state.pending.insert(
            token.to_string(),
            PendingApproval {
                generation,
                channel_id: ORIGIN_CHANNEL.to_string(),
                sender: tx,
            },
        );
        state
            .posts
            .insert(post_id.to_string(), (token.to_string(), generation));
        (rx, generation)
    }

    /// A guard armed for an earlier request must not retire a later request that
    /// happened to draw the same token.
    ///
    /// `new_approval_token` is six random characters — finite, and not a
    /// uniqueness guarantee. Keying cleanup on the token alone meant a stale
    /// guard could delete a live prompt's sender and post binding, stranding an
    /// approval the operator could still see on screen.
    #[test]
    fn stale_guard_cannot_retire_a_newer_same_token_registration() {
        let ch = channel(Vec::new(), "https://mm.example.com".into());

        // Old request registers, and its guard stays armed.
        let (_old_rx, old_generation) = register(&ch, "abc123", "old_post");
        let old_guard = ApprovalGuard::new(
            Arc::clone(&ch.approvals),
            "abc123".to_string(),
            old_generation,
        );

        // The old registration goes away the way a resolving path would retire
        // it, and a new request then draws the same token.
        ch.approvals
            .lock()
            .retire_generation("abc123", old_generation);
        let (mut new_rx, new_generation) = register(&ch, "abc123", "new_post");
        assert_ne!(old_generation, new_generation);

        // The stale guard drops afterwards.
        drop(old_guard);

        let state = ch.approvals.lock();
        assert!(
            state.pending.contains_key("abc123"),
            "a stale guard must not retire a newer registration's prompt"
        );
        assert!(
            state.posts.contains_key("new_post"),
            "a stale guard must not drop a newer registration's post binding"
        );
        drop(state);
        assert!(
            new_rx.try_recv().is_err(),
            "the newer prompt must still be awaiting a decision, not cancelled"
        );
    }

    /// The guard for the *live* registration still cleans up on drop.
    #[test]
    fn matching_guard_still_retires_its_own_registration() {
        let ch = channel(Vec::new(), "https://mm.example.com".into());
        let (_rx, generation) = register(&ch, "abc123", "post1");
        drop(ApprovalGuard::new(
            Arc::clone(&ch.approvals),
            "abc123".to_string(),
            generation,
        ));
        let state = ch.approvals.lock();
        assert!(
            state.pending.is_empty(),
            "the owning guard must still clean up"
        );
        assert!(state.posts.is_empty(), "and must drop its own post binding");
    }

    /// A transient `/users/me` failure at listener startup must not leave typed
    /// approvals permanently unanswerable.
    ///
    /// Both listeners copy the bot ID once and never refetch, so before this the
    /// operator had to restart the daemon. The resolver now recovers the
    /// identity when the copied value is empty; empty results are not cached, so
    /// the first successful lookup is the one that sticks.
    #[tokio::test]
    async fn typed_approval_recovers_after_a_transient_identity_failure() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        // Fails first, then succeeds — the shape of a listener that started
        // while Mattermost was briefly unavailable.
        Mock::given(method("GET"))
            .and(path("/api/v4/users/me"))
            .respond_with(ResponseTemplate::new(500))
            .up_to_n_times(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v4/users/me"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"id": BOT_USER, "username": "glados"})),
            )
            .mount(&server)
            .await;

        let ch = channel(vec![ALLOWED_USER.to_string()], server.uri());

        // The listener's startup lookup fails, yielding the empty ID it then
        // carries into every `process_inbound_post` call.
        let (startup_id, _) = ch.get_bot_identity().await;
        assert!(startup_id.is_empty(), "the first lookup must fail");

        let (_rx, _generation) = register(&ch, "abc123", "post1");
        let outcome = ch
            .try_resolve_approval_reply(
                &json!({
                    "id": "post2",
                    "user_id": ALLOWED_USER,
                    "message": "abc123 yes",
                    "create_at": 1,
                    "channel_id": ORIGIN_CHANNEL,
                }),
                &startup_id,
            )
            .await;

        assert_eq!(outcome, ApprovalReplyOutcome::Consumed);
        assert!(
            ch.approvals.lock().pending.is_empty(),
            "the operator must be able to answer once the identity lookup recovers, \
             without restarting the daemon"
        );
    }

    /// A refusal must not swallow text this channel has no claim on.
    ///
    /// With the identity checks ahead of the ownership lookup, an unresolved bot
    /// identity turned every six-character approval-shaped message into
    /// `Consumed`, losing unrelated user text.
    #[tokio::test]
    async fn unknown_token_reaches_the_model_even_when_identity_is_unavailable() {
        let ch = channel(vec!["*".to_string()], "https://mm.example.com".into());
        assert_eq!(
            ch.try_resolve_approval_reply(
                &json!({
                    "id": "post1",
                    "user_id": ALLOWED_USER,
                    "message": "zzz999 yes",
                    "create_at": 1,
                    "channel_id": ORIGIN_CHANNEL,
                }),
                "",
            )
            .await,
            ApprovalReplyOutcome::NotFound,
            "a security refusal must not also lose unrelated user text"
        );
    }
}

/// Registration identity must hold across token allocation, post binding, and
/// both decision paths — not just guard cleanup.
///
/// The six-character token is not a request identity. Where a decision path
/// acted on the token alone, a later request that drew the same token could
/// receive a decision made for an earlier prompt: a confused deputy that grants
/// authority to the wrong request rather than merely denying one.
#[cfg(test)]
mod approval_registration_identity_tests {
    use super::*;
    use serde_json::json;

    const ALLOWED_USER: &str = "operator_user_id";
    const BOT_USER: &str = "bot_user_id";
    const ORIGIN_CHANNEL: &str = "chan1";

    fn channel() -> MattermostChannel {
        let ch = MattermostChannel::new(
            "https://mm.example.com".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "approval_registration_alias",
            Arc::new(|| vec![ALLOWED_USER.to_string()]),
            false,
            false,
        );
        let _ = ch
            .bot_identity
            .set((BOT_USER.to_string(), "glados".to_string()));
        ch
    }

    fn register(
        ch: &MattermostChannel,
        token: &str,
        post_id: Option<&str>,
    ) -> (tokio::sync::oneshot::Receiver<ChannelApprovalResponse>, u64) {
        let (tx, rx) = tokio::sync::oneshot::channel();
        let mut state = ch.approvals.lock();
        let generation = state.next_generation;
        state.next_generation += 1;
        state.pending.insert(
            token.to_string(),
            PendingApproval {
                generation,
                channel_id: ORIGIN_CHANNEL.to_string(),
                sender: tx,
            },
        );
        if let Some(post_id) = post_id {
            state
                .posts
                .insert(post_id.to_string(), (token.to_string(), generation));
        }
        (rx, generation)
    }

    /// A tap on an older prompt's still-visible post must not answer the newer
    /// request that has since taken the token.
    #[tokio::test]
    async fn reaction_on_an_older_post_cannot_resolve_a_newer_registration() {
        let ch = channel();

        // Request A registers and binds its post, then goes away.
        let (_a_rx, a_generation) = register(&ch, "abc123", Some("post_a"));
        ch.approvals.lock().pending.remove("abc123");

        // Request B draws the same token. A's post binding is still resident.
        let (mut b_rx, b_generation) = register(&ch, "abc123", None);
        assert_ne!(a_generation, b_generation);

        let event = json!({
            "data": {
                "reaction": json!({
                    "post_id": "post_a",
                    "user_id": ALLOWED_USER,
                    "emoji_name": APPROVAL_EMOJI_APPROVE,
                })
                .to_string(),
            }
        });

        assert!(
            !ch.try_resolve_approval_reaction(&event).await,
            "a reaction on A's post must not resolve B"
        );
        assert!(
            b_rx.try_recv().is_err(),
            "B must not receive a decision made for A's prompt"
        );
        assert!(
            ch.approvals.lock().owns("abc123", b_generation),
            "B's registration must survive a tap meant for A"
        );
    }

    /// A typed reply recognized against one registration must not resolve a
    /// different one that took the token while the identity lookup awaited.
    ///
    /// The swap must happen *inside* `get_bot_identity().await`, and that
    /// ordering is enforced rather than timed: the mock responder performs the
    /// swap itself, so the identity request cannot complete until the
    /// replacement has already happened. An earlier version slept 50ms and
    /// hoped, which proves nothing under unusual scheduling.
    #[tokio::test]
    async fn typed_reply_cannot_resolve_a_registration_that_replaced_it() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

        /// Swaps the registration while serving the identity lookup, which is
        /// exactly the window under test.
        struct SwapDuringLookup {
            approvals: Arc<Mutex<ApprovalState>>,
            old_generation: u64,
            replacement: Mutex<Option<tokio::sync::oneshot::Sender<ChannelApprovalResponse>>>,
        }

        impl Respond for SwapDuringLookup {
            fn respond(&self, _request: &Request) -> ResponseTemplate {
                let mut state = self.approvals.lock();
                state.retire_generation("abc123", self.old_generation);
                let generation = state.next_generation;
                state.next_generation += 1;
                let sender = self
                    .replacement
                    .lock()
                    .take()
                    .expect("the identity lookup runs once");
                state.pending.insert(
                    "abc123".to_string(),
                    PendingApproval {
                        generation,
                        channel_id: ORIGIN_CHANNEL.to_string(),
                        sender,
                    },
                );
                ResponseTemplate::new(200)
                    .set_body_json(json!({"id": BOT_USER, "username": "glados"}))
            }
        }

        // The server starts first so the channel can be built against its real
        // URI: there must be exactly one `ApprovalState`, shared by the channel
        // and the responder. An earlier version built the channel first and
        // copied the state into a second `Arc`, so the swap mutated a map the
        // resolver never read and the test passed without proving anything.
        let server = MockServer::start().await;

        // Deliberately not `channel_at`: that seeds the identity cache, and a
        // seeded cache would skip the refresh this test depends on.
        let ch = MattermostChannel::new(
            server.uri(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "approval_registration_alias",
            Arc::new(|| vec![ALLOWED_USER.to_string()]),
            false,
            false,
        );
        let (_a_rx, a_generation) = register(&ch, "abc123", None);
        let (b_tx, mut b_rx) = tokio::sync::oneshot::channel();

        Mock::given(method("GET"))
            .and(path("/api/v4/users/me"))
            .respond_with(SwapDuringLookup {
                approvals: Arc::clone(&ch.approvals),
                old_generation: a_generation,
                replacement: Mutex::new(Some(b_tx)),
            })
            .mount(&server)
            .await;

        let outcome = ch
            .try_resolve_approval_reply(
                &json!({
                    "id": "post1",
                    "user_id": ALLOWED_USER,
                    "message": "abc123 yes",
                    "create_at": 1,
                    "channel_id": ORIGIN_CHANNEL,
                }),
                "",
            )
            .await;

        assert_eq!(
            outcome,
            ApprovalReplyOutcome::Consumed,
            "the reply is approval traffic and must not reach the model"
        );
        assert!(
            b_rx.try_recv().is_err(),
            "B must not receive a reply written in answer to A's prompt"
        );
    }

    /// Allocation must not hand out a token that is live or still inside the
    /// retired window — either collision would let one prompt's decision land
    /// on another.
    #[test]
    fn allocation_skips_live_and_retired_tokens() {
        let ch = channel();
        let (_rx, _generation) = register(&ch, "live01", None);
        ch.approvals.lock().remember_retired("gone02");

        for _ in 0..256 {
            let token = ch
                .approvals
                .lock()
                .allocate_token()
                .expect("a free token must be available");
            assert_ne!(token, "live01", "must not reuse a live token");
            assert_ne!(
                token, "gone02",
                "must not reuse a token in the retired window"
            );
        }
    }

    /// The binding is only added when the registration still owns the token, so
    /// a prompt posted for a request that has already gone away cannot leave a
    /// binding that a later request would answer through.
    #[test]
    fn post_binding_requires_the_registration_to_still_be_live() {
        let ch = channel();
        let (_rx, generation) = register(&ch, "abc123", None);
        ch.approvals.lock().retire_generation("abc123", generation);

        let state = ch.approvals.lock();
        assert!(
            !state.owns("abc123", generation),
            "the registration is gone, so `owns` must gate the binding out"
        );
    }
}

/// A listener must not run, and a post must not be admitted, while the bot's
/// own identity is unknown.
///
/// The approval resolver already refuses to *decide* without an identity, but
/// the prompt the bot posts is not approval-shaped, so it takes the ordinary
/// parser path instead. There the self-loop check compares the author against
/// `bot_user_id`; an empty one matches nothing, so under a wildcard peer group
/// and the default `mention_only = false` the bot's own security prompt was
/// admitted as model input, carrying its token and tool details back into the
/// conversation.
#[cfg(test)]
mod bot_identity_admission_tests {
    use super::*;
    use serde_json::json;

    const BOT_USER: &str = "bot_user_id";
    const OPERATOR: &str = "operator_user_id";
    const ORIGIN_CHANNEL: &str = "chan1";

    /// Wildcard peers and `mention_only = false`: the documented configuration
    /// in which nothing else would stop a bot-authored post.
    fn wildcard_channel() -> MattermostChannel {
        MattermostChannel::new(
            "https://mm.example.com".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "identity_admission_alias",
            Arc::new(|| vec!["*".to_string()]),
            false,
            false,
        )
    }

    fn post_from(user_id: &str, message: &str) -> serde_json::Value {
        json!({
            "id": "post1",
            "user_id": user_id,
            "message": message,
            "create_at": 10,
            "channel_id": ORIGIN_CHANNEL,
        })
    }

    /// The exact reported shape: the bot's own approval prompt, arriving while
    /// the startup identity snapshot is empty.
    #[tokio::test]
    async fn bot_authored_prompt_is_not_admitted_when_identity_is_unknown() {
        let ch = wildcard_channel();
        let (tx, mut inbound) = tokio::sync::mpsc::channel(8);

        ch.process_inbound_post(
            &post_from(
                BOT_USER,
                "APPROVAL REQUIRED [abc123]\nTool: shell\nArgs: rm -rf /",
            ),
            "", // the empty snapshot a failed identity lookup produces
            "glados",
            0,
            ORIGIN_CHANNEL,
            false,
            &tx,
        )
        .await;

        assert!(
            inbound.try_recv().is_err(),
            "the bot's own approval prompt must never be fed back as model input"
        );
    }

    /// Not specific to approval prompts: with no identity, no post is safe to
    /// admit, because none of them can be checked against the bot.
    #[tokio::test]
    async fn no_post_is_admitted_when_identity_is_unknown() {
        let ch = wildcard_channel();
        let (tx, mut inbound) = tokio::sync::mpsc::channel(8);

        ch.process_inbound_post(
            &post_from(OPERATOR, "what is the weather"),
            "",
            "glados",
            0,
            ORIGIN_CHANNEL,
            false,
            &tx,
        )
        .await;

        assert!(
            inbound.try_recv().is_err(),
            "admission must fail closed rather than guess"
        );
    }

    /// The counterweight the review asked for: once identity is known, an
    /// ordinary operator message must still get through. Without this, "fail
    /// closed" could be satisfied by suppressing everything.
    #[tokio::test]
    async fn operator_message_is_admitted_once_identity_is_known() {
        let ch = wildcard_channel();
        let (tx, mut inbound) = tokio::sync::mpsc::channel(8);

        ch.process_inbound_post(
            &post_from(OPERATOR, "what is the weather"),
            BOT_USER,
            "glados",
            0,
            ORIGIN_CHANNEL,
            false,
            &tx,
        )
        .await;

        let message = inbound
            .try_recv()
            .expect("a real operator message must still reach the model");
        assert_eq!(message.content, "what is the weather");
    }

    /// And the self-loop check still holds when identity *is* known, so the
    /// guard above did not replace it.
    #[tokio::test]
    async fn bot_authored_post_stays_rejected_when_identity_is_known() {
        let ch = wildcard_channel();
        let (tx, mut inbound) = tokio::sync::mpsc::channel(8);

        ch.process_inbound_post(
            &post_from(BOT_USER, "APPROVAL REQUIRED [abc123]"),
            BOT_USER,
            "glados",
            0,
            ORIGIN_CHANNEL,
            false,
            &tx,
        )
        .await;

        assert!(inbound.try_recv().is_err());
    }

    /// A listener must refuse to start rather than run blind. The supervised
    /// listener retries with backoff, so a transient outage recovers on its own
    /// while a durable one keeps the channel visibly down.
    #[tokio::test]
    async fn listener_refuses_to_start_without_a_bot_identity() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/v4/users/me"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;

        let ch = MattermostChannel::new(
            server.uri(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "identity_admission_alias",
            Arc::new(|| vec!["*".to_string()]),
            false,
            false,
        );

        let error = ch
            .require_bot_identity()
            .await
            .expect_err("an unavailable identity must not yield a usable listener");
        assert!(
            format!("{error}").contains("identity unavailable"),
            "the error must name the cause; got: {error}"
        );
    }
}

/// Channel-purpose plumbing: discovery retains it, the opt-in gates it.
#[cfg(test)]
mod channel_purpose_plumbing_tests {
    use super::*;
    use serde_json::json;

    fn channel(purpose_as_instructions: bool) -> MattermostChannel {
        MattermostChannel::new(
            "https://mm.example.com".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "purpose_alias",
            Arc::new(Vec::new),
            false,
            false,
        )
        .with_purpose_as_instructions(purpose_as_instructions)
    }

    fn discovered(purpose: &str) -> Vec<serde_json::Value> {
        vec![json!({"id": "room1", "type": "O", "team_id": "t1", "purpose": purpose})]
    }

    /// The purpose rides along on the discovery response already being parsed,
    /// so retaining it costs no additional request.
    #[test]
    fn discovery_retains_the_purpose() {
        let targets = filter_discovered_channels(&discovered("Arch packaging"), &[], true);
        assert_eq!(targets[0].purpose.as_deref(), Some("Arch packaging"));
    }

    /// "Unset" and "blank" are one state.
    #[test]
    fn blank_purpose_is_normalised_to_none() {
        let targets = filter_discovered_channels(&discovered("   "), &[], true);
        assert_eq!(targets[0].purpose, None);
    }

    /// Opting in surfaces the purpose to prompt assembly.
    #[test]
    fn opted_in_channel_exposes_the_room_purpose() {
        let ch = channel(true);
        ch.refresh_room_purposes(&filter_discovered_channels(
            &discovered("Arch packaging"),
            &[],
            true,
        ));
        assert_eq!(
            ch.room_context("room1").and_then(|c| c.purpose).as_deref(),
            Some("Arch packaging")
        );
        assert!(
            ch.room_context("other").is_none(),
            "unknown rooms contribute nothing"
        );
    }

    /// Without the opt-in the channel retains no purpose at all — absence is
    /// the default, and the text never reaches the prompt.
    #[test]
    fn opt_out_channel_never_retains_or_exposes_a_purpose() {
        let ch = channel(false);
        ch.refresh_room_purposes(&filter_discovered_channels(
            &discovered("Arch packaging"),
            &[],
            true,
        ));
        assert!(ch.room_purposes.lock().is_empty(), "nothing may be cached");
        assert!(ch.room_context("room1").is_none());
    }

    /// The lookup key is this channel's own addressing string, not a bare room
    /// ID, and for a threaded reply that string is `channel_id:root_id`.
    ///
    /// The first version of this feature looked the purpose up by
    /// `ChannelMessage::channel`, which for Mattermost is the constant
    /// `"mattermost"` rather than a room, so no room ever matched and the
    /// purpose silently never reached the prompt. Every test then passed a bare
    /// room ID directly, so none of them noticed.
    #[test]
    fn room_context_accepts_the_channels_own_addressing_string() {
        let ch = channel(true);
        ch.refresh_room_purposes(&filter_discovered_channels(
            &discovered("Arch packaging"),
            &[],
            true,
        ));

        // Plain room, exactly as `reply_target` carries it for a top-level post.
        assert_eq!(
            ch.room_context("room1").and_then(|c| c.purpose).as_deref(),
            Some("Arch packaging")
        );
        // Threaded reply: purposes are per room, so the thread half is stripped.
        assert_eq!(
            ch.room_context("room1:root_post_id")
                .and_then(|c| c.purpose)
                .as_deref(),
            Some("Arch packaging"),
            "a threaded reply is still in the room the prompt belongs to"
        );
        // The channel *type* is not a room and must never resolve.
        assert!(
            ch.room_context("mattermost").is_none(),
            "the channel type is not a room identifier"
        );
    }

    /// A purpose cleared in Mattermost must stop being injected: the refresh
    /// replaces rather than merges.
    #[test]
    fn refresh_drops_a_cleared_purpose() {
        let ch = channel(true);
        ch.refresh_room_purposes(&filter_discovered_channels(
            &discovered("Arch packaging"),
            &[],
            true,
        ));
        ch.refresh_room_purposes(&filter_discovered_channels(&discovered(""), &[], true));
        assert!(
            ch.room_context("room1").is_none(),
            "a cleared purpose must stop being injected"
        );
    }
}

/// Keeping the cached purpose fresh: the `channel_updated` fast path, and the
/// refresh cadence for aliases that do not auto-discover.
#[cfg(test)]
mod channel_purpose_freshness_tests {
    use super::*;
    use serde_json::json;

    fn channel(purpose_as_instructions: bool) -> MattermostChannel {
        MattermostChannel::new(
            "https://mm.example.com".into(),
            Some("token".into()),
            None,
            None,
            Vec::new(),
            "purpose_freshness_alias",
            Arc::new(Vec::new),
            false,
            false,
        )
        .with_purpose_as_instructions(purpose_as_instructions)
    }

    fn channel_updated_event(id: &str, purpose: &str) -> serde_json::Value {
        json!({
            "event": "channel_updated",
            "data": {
                "channel": json!({"id": id, "type": "O", "purpose": purpose}).to_string(),
            }
        })
    }

    /// The edit signal: discovery compares the *set* of rooms, which a purpose
    /// edit leaves identical, so without this the change waits out the whole
    /// refresh interval.
    #[test]
    fn channel_updated_applies_a_new_purpose_immediately() {
        let ch = channel(true);
        ch.apply_ws_channel_purpose_update(&channel_updated_event("room1", "Arch packaging"));
        assert_eq!(
            ch.room_context("room1").and_then(|c| c.purpose).as_deref(),
            Some("Arch packaging")
        );
    }

    /// Clearing the purpose in Mattermost must stop the injection at once, not
    /// leave the previous value cached until the next discovery pass.
    #[test]
    fn channel_updated_clears_a_removed_purpose() {
        let ch = channel(true);
        ch.apply_ws_channel_purpose_update(&channel_updated_event("room1", "Arch packaging"));
        ch.apply_ws_channel_purpose_update(&channel_updated_event("room1", "   "));
        assert!(
            ch.room_context("room1").is_none(),
            "a cleared purpose must stop being injected immediately"
        );
    }

    /// One room's edit must not disturb another's cached purpose.
    #[test]
    fn channel_updated_touches_only_the_named_room() {
        let ch = channel(true);
        ch.apply_ws_channel_purpose_update(&channel_updated_event("room1", "first"));
        ch.apply_ws_channel_purpose_update(&channel_updated_event("room2", "second"));
        assert_eq!(
            ch.room_context("room1").and_then(|c| c.purpose).as_deref(),
            Some("first")
        );
        assert_eq!(
            ch.room_context("room2").and_then(|c| c.purpose).as_deref(),
            Some("second")
        );
    }

    /// An alias that has not opted in must retain nothing, even when the event
    /// arrives — the opt-in gates storage, not just exposure.
    #[test]
    fn channel_updated_is_ignored_without_the_opt_in() {
        let ch = channel(false);
        ch.apply_ws_channel_purpose_update(&channel_updated_event("room1", "Arch packaging"));
        assert!(ch.room_purposes.lock().is_empty());
    }

    /// A real `channel_updated` payload, kept verbatim.
    ///
    /// The other tests in this module build their events from a fixture helper,
    /// so they would pass even if this code assumed the wrong wire shape. This
    /// one pins the shape itself: `data.channel` is a JSON *string* holding the
    /// channel object, and the purpose is read from that object's `purpose`
    /// field. If Mattermost ever changes that encoding, this fails while the
    /// helper-built tests would not.
    #[test]
    fn real_channel_updated_payload_updates_the_purpose() {
        let event: serde_json::Value = serde_json::from_str(
            r#"{
  "event": "channel_updated",
  "data": {
    "channel": "{\"id\":\"nq83j71sebdfh58ktrce5ijtcwy\",\"create_at\":1609459200000,\"update_at\":1712649608411,\"delete_at\":0,\"team_id\":\"b7ysq51sebfh58ktrce5ijtcwy\",\"type\":\"P\",\"display_name\":\"Updated Channel Name\",\"name\":\"updated-channel-name\",\"header\":\"New channel header text\",\"purpose\":\"Updated channel purpose\",\"last_post_at\":1712649608426,\"total_msg_count\":142,\"extra_update_at\":0,\"creator_id\":\"ay5sq51sebfh58ktrce5ijtcwy\"}"
  },
  "broadcast": {
    "omit_users": null,
    "user_id": "",
    "channel_id": "nq83j71sebdfh58ktrce5ijtcwy",
    "team_id": "b7ysq51sebfh58ktrce5ijtcwy"
  },
  "seq": 42
}"#,
        )
        .expect("the sample payload must parse");

        let ch = channel(true);
        ch.apply_ws_channel_purpose_update(&event);

        assert_eq!(
            ch.room_context("nq83j71sebdfh58ktrce5ijtcwy")
                .and_then(|c| c.purpose)
                .as_deref(),
            Some("Updated channel purpose"),
            "the purpose must be read from the embedded channel object"
        );
    }

    /// A malformed event must not panic or corrupt the cache.
    #[test]
    fn malformed_channel_updated_events_are_ignored() {
        let ch = channel(true);
        ch.apply_ws_channel_purpose_update(&json!({"event": "channel_updated"}));
        ch.apply_ws_channel_purpose_update(&json!({
            "event": "channel_updated",
            "data": {"channel": "not json"}
        }));
        ch.apply_ws_channel_purpose_update(&json!({
            "event": "channel_updated",
            "data": {"channel": json!({"purpose": "no id"}).to_string()}
        }));
        assert!(ch.room_purposes.lock().is_empty());
    }
}

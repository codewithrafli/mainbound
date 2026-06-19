use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

const KEYRING_SERVICE: &str = "dev.mainbound.app";
const KEYRING_USER: &str = "groq-api-key";
const GROQ_TRANSCRIPTIONS_API: &str = "https://api.groq.com/openai/v1/audio/transcriptions";

fn http() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("mainbound")
            .build()
            .expect("reqwest client")
    })
}

fn groq_entry() -> AppResult<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, KEYRING_USER)
        .map_err(|e| AppError::Pty(format!("keychain: {e}")))
}

fn groq_key_cache() -> &'static Mutex<Option<String>> {
    static CACHE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

fn groq_key() -> AppResult<String> {
    if let Some(key) = groq_key_cache()
        .lock()
        .map_err(|e| AppError::Pty(format!("groq key cache: {e}")))?
        .clone()
    {
        return Ok(key);
    }

    let key = groq_entry()?
        .get_password()
        .map_err(|_| AppError::Pty("Groq API key is not configured".into()))?;
    *groq_key_cache()
        .lock()
        .map_err(|e| AppError::Pty(format!("groq key cache: {e}")))? = Some(key.clone());
    Ok(key)
}

#[derive(Serialize)]
pub struct SpeechKeyStatus {
    configured: bool,
}

#[derive(Deserialize)]
struct GroqTranscription {
    text: String,
}

#[tauri::command]
pub fn speech_groq_key_status() -> SpeechKeyStatus {
    SpeechKeyStatus {
        configured: groq_key_cache()
            .lock()
            .map(|key| key.is_some())
            .unwrap_or(false),
    }
}

#[tauri::command]
pub fn speech_groq_set_key(key: String) -> AppResult<()> {
    let key = key.trim();
    if key.is_empty() {
        return Err(AppError::Pty("Groq API key cannot be empty".into()));
    }
    groq_entry()?
        .set_password(key)
        .map_err(|e| AppError::Pty(format!("keychain: {e}")))?;
    *groq_key_cache()
        .lock()
        .map_err(|e| AppError::Pty(format!("groq key cache: {e}")))? = Some(key.to_string());
    Ok(())
}

#[tauri::command]
pub fn speech_groq_clear_key() -> AppResult<()> {
    let entry = groq_entry()?;
    let _ = entry.delete_credential();
    if let Ok(mut key) = groq_key_cache().lock() {
        *key = None;
    }
    Ok(())
}

#[tauri::command]
pub async fn speech_groq_transcribe(
    audio: Vec<u8>,
    mime: String,
    language: Option<String>,
) -> AppResult<String> {
    if audio.is_empty() {
        return Err(AppError::Pty("No audio was recorded".into()));
    }

    let token = groq_key()?;
    let extension = if mime.contains("ogg") {
        "ogg"
    } else if mime.contains("wav") {
        "wav"
    } else if mime.contains("mp4") {
        "m4a"
    } else {
        "webm"
    };
    let file_part = reqwest::multipart::Part::bytes(audio)
        .file_name(format!("dictation.{extension}"))
        .mime_str(&mime)
        .map_err(|e| AppError::Pty(format!("audio mime: {e}")))?;
    let mut form = reqwest::multipart::Form::new()
        .text("model", "whisper-large-v3")
        .text("response_format", "json")
        .text("temperature", "0")
        .text(
            "prompt",
            "Transcribe exactly what the speaker says. The speaker may mix Indonesian and English developer terms. Do not translate, summarize, autocomplete, invent examples, or add words that were not spoken. Preserve technical terms such as framework, repository, GitHub, branch, commit, pull request, TypeScript, JavaScript, React, Vue, Nuxt, Next.js, Node.js, backend, frontend, deploy. Output only the transcript.",
        )
        .part("file", file_part);

    if let Some(language) = language.filter(|value| !value.trim().is_empty()) {
        form = form.text("language", language);
    }

    let resp = http()
        .post(GROQ_TRANSCRIPTIONS_API)
        .bearer_auth(token)
        .multipart(form)
        .send()
        .await
        .map_err(|e| AppError::Pty(format!("groq: {e}")))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| AppError::Pty(format!("groq: {e}")))?;
    if !status.is_success() {
        return Err(AppError::Pty(format!("groq ({status}): {body}")));
    }
    let transcription: GroqTranscription =
        serde_json::from_str(&body).map_err(|e| AppError::Pty(format!("groq response: {e}")))?;
    Ok(transcription.text.trim().to_string())
}

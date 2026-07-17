use std::{env, fs, io::Cursor, path::PathBuf};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use minisign::{sign, SecretKeyBox};
use minisign_verify::{PublicKey as VerifyPublicKey, Signature};
use serde::Serialize;

#[derive(Debug)]
struct Options {
    installer: PathBuf,
    signature: PathBuf,
    manifest: PathBuf,
    version: String,
    url: String,
    notes: Option<String>,
    published_at: String,
}

#[derive(Serialize)]
struct Manifest {
    version: String,
    notes: Option<String>,
    pub_date: String,
    platforms: Platforms,
}

#[derive(Serialize)]
struct Platforms {
    #[serde(rename = "windows-x86_64")]
    windows_x86_64: Platform,
}

#[derive(Serialize)]
struct Platform {
    signature: String,
    url: String,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("update packager: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().is_some_and(|argument| argument == "verify") {
        if args.len() != 5 || args[1] != "--installer" || args[3] != "--signature" {
            return Err("用法: verify --installer <path> --signature <path>".to_string());
        }
        let installer = fs::read(&args[2]).map_err(|error| format!("读取安装包失败: {error}"))?;
        let signature =
            fs::read_to_string(&args[4]).map_err(|error| format!("读取签名失败: {error}"))?;
        return verify_production_signature(&installer, &signature);
    }
    let options = parse_args(args)?;
    let private_key = env::var("TAURI_SIGNING_PRIVATE_KEY")
        .map_err(|_| "缺少 TAURI_SIGNING_PRIVATE_KEY".to_string())?;
    let password = env::var("TAURI_SIGNING_PRIVATE_KEY_PASSWORD").ok();
    package_with_public_key(&options, &private_key, password, UPDATE_PUBLIC_KEY)
}

fn parse_args(args: impl IntoIterator<Item = String>) -> Result<Options, String> {
    let mut args = args.into_iter();
    let mut value = |name: &str| {
        let flag = args.next().ok_or_else(|| format!("缺少参数 {name}"))?;
        if flag != name {
            return Err(format!("预期参数 {name}，实际为 {flag}"));
        }
        args.next().ok_or_else(|| format!("缺少 {name} 的值"))
    };
    let installer = PathBuf::from(value("--installer")?);
    let signature = PathBuf::from(value("--signature")?);
    let manifest = PathBuf::from(value("--manifest")?);
    let version = value("--version")?;
    let url = value("--url")?;
    let notes_path = value("--notes")?;
    let published_at = value("--published-at")?;
    if args.next().is_some() {
        return Err("存在未知的额外参数".to_string());
    }
    let notes = if notes_path.is_empty() {
        None
    } else {
        Some(fs::read_to_string(notes_path).map_err(|error| format!("读取更新说明失败: {error}"))?)
    };
    Ok(Options {
        installer,
        signature,
        manifest,
        version,
        url,
        notes,
        published_at,
    })
}

fn package_with_public_key(
    options: &Options,
    private_key: &str,
    password: Option<String>,
    public_key: &str,
) -> Result<(), String> {
    let installer = fs::read(&options.installer)
        .map_err(|error| format!("读取安装包失败 {}: {error}", options.installer.display()))?;
    let key_text = decode_private_key(private_key)?;
    let secret_key_box = SecretKeyBox::from_string(&key_text)
        .map_err(|error| format!("解析签名私钥失败: {error}"))?;
    let secret_key = match password {
        Some(password) => secret_key_box.into_secret_key(Some(password)),
        None => secret_key_box.into_unencrypted_secret_key(),
    }
    .map_err(|error| format!("加载签名私钥失败: {error}"))?;
    let signature = sign(
        None,
        &secret_key,
        Cursor::new(&installer),
        Some(&format!("SSH Router {}", options.version)),
        None,
    )
    .map_err(|error| format!("签名安装包失败: {error}"))?
    .into_string();
    verify_signature(&installer, &signature, public_key)?;
    fs::write(&options.signature, &signature).map_err(|error| format!("写入签名失败: {error}"))?;
    let manifest = Manifest {
        version: options.version.clone(),
        notes: options.notes.clone(),
        pub_date: options.published_at.clone(),
        platforms: Platforms {
            windows_x86_64: Platform {
                signature,
                url: options.url.clone(),
            },
        },
    };
    let json = serde_json::to_vec_pretty(&manifest)
        .map_err(|error| format!("生成 latest.json 失败: {error}"))?;
    fs::write(&options.manifest, json).map_err(|error| format!("写入 latest.json 失败: {error}"))
}

const UPDATE_PUBLIC_KEY: &str = "RWRCIe7UU5heZhJWzr8REXRosR9lKZktyS1xnM5mMmCy0wwdfxUgZLTo";

fn verify_production_signature(contents: &[u8], signature: &str) -> Result<(), String> {
    verify_signature(contents, signature, UPDATE_PUBLIC_KEY)
}

fn verify_signature(contents: &[u8], signature: &str, public_key: &str) -> Result<(), String> {
    let public_key = VerifyPublicKey::from_base64(public_key)
        .map_err(|error| format!("加载生产更新公钥失败: {error}"))?;
    let signature =
        Signature::decode(signature).map_err(|error| format!("签名格式无效: {error}"))?;
    public_key
        .verify(contents, &signature, false)
        .map_err(|error| format!("签名不匹配生产公钥: {error}"))
}

fn decode_private_key(value: &str) -> Result<String, String> {
    if value.contains("untrusted comment:") {
        return Ok(value.to_string());
    }
    let decoded = STANDARD
        .decode(value.trim())
        .map_err(|error| format!("签名私钥不是有效文本或 Base64: {error}"))?;
    String::from_utf8(decoded).map_err(|error| format!("签名私钥不是 UTF-8: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use minisign::{verify, KeyPair, PublicKey, SignatureBox};

    #[test]
    fn creates_compatible_manifest_and_signature() {
        let directory = tempfile::tempdir().expect("应创建测试目录");
        let installer = directory.path().join("setup.exe");
        let signature = directory.path().join("setup.exe.sig");
        let manifest = directory.path().join("latest.json");
        fs::write(&installer, b"installer").expect("应写入测试安装包");
        let key_pair = KeyPair::generate_unencrypted_keypair().expect("应生成测试密钥");
        let key_text = key_pair.sk.to_box(None).expect("应导出私钥").into_string();
        let options = Options {
            installer,
            signature: signature.clone(),
            manifest: manifest.clone(),
            version: "0.0.9".to_string(),
            url: "https://example.test/setup.exe".to_string(),
            notes: Some("更新说明".to_string()),
            published_at: "2026-07-17T00:00:00Z".to_string(),
        };

        let public_key = PublicKey::from_secret_key(&key_pair.sk).expect("应恢复公钥");
        package_with_public_key(&options, &key_text, None, &public_key.to_base64())
            .expect("应生成更新产物");

        let signature_box = SignatureBox::from_file(signature).expect("签名应可解析");
        verify(
            &public_key,
            &signature_box,
            Cursor::new(b"installer"),
            true,
            false,
            false,
        )
        .expect("签名应有效");
        let production_verifier_error = ssh_router_app_core::signature::verify_installer(
            b"installer",
            &fs::read_to_string(&options.signature).expect("应读取签名"),
        )
        .expect_err("测试密钥不应匹配生产公钥");
        assert!(!production_verifier_error.to_string().contains("格式无效"));
        let json: serde_json::Value =
            serde_json::from_slice(&fs::read(manifest).expect("应读取元数据"))
                .expect("元数据应为 JSON");
        assert_eq!(json["version"], "0.0.9");
        assert_eq!(
            json["platforms"]["windows-x86_64"]["url"],
            "https://example.test/setup.exe"
        );
    }
}

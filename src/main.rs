use actix_web::{App, get, HttpServer, web, Responder, put};
use actix_web::http::StatusCode;
use lazy_static::lazy_static;
use protobuf::Message as _;
use crate::ota_proto::ota_metadata::ota_metadata_v1::{AbInstallType, OtaType};
use crate::ota_proto::ota_metadata::{AbConfig, InstallConfig, OtaMetadataV1};
use spin::Mutex;
use std::collections::HashMap;
use serde::{Deserialize, Serialize};

mod ota_proto {
    include!(concat!(env!("OUT_DIR"), "/proto/mod.rs"));
}

const SUPPORTED_DEVICES: &[&str] = &["venus", "sunny", "curtana", "alioth", "lemonadep", "oriole", "raven", "sake"];

#[derive(Deserialize, Serialize)]
struct BuildMetadata {
    device: String,
    filename: String,
    url: String,
    size: i64,
}

lazy_static! {
    // TODO: commit to disk.
    static ref BUILD_MAPPING: Mutex<HashMap<String, BuildMetadata>> = Mutex::new(HashMap::new());
}

fn build_response_proto(device_name: &str) -> OtaMetadataV1 {
    let install_config = InstallConfig {
        wipe: false,
        retrofit_dynamic_partitions: false,
        ..Default::default()
    };
    let ab_config = AbConfig {
        force_switch_slot: false,
        ..Default::default()
    };
    let guard = BUILD_MAPPING.lock();
    let metadata = guard.get(device_name).unwrap();
    // all devices are A/B so we don't need to handle a-only.
    OtaMetadataV1 {
        name: metadata.filename.clone(),
        url: metadata.url.clone(),
        changelog_url: "".to_string(),
        size_bytes: metadata.size,
        type_: OtaType::AB.into(),
        ab_install_type: AbInstallType::NON_STREAMING.into(),
        ab_config: Some(ab_config).into(),
        install_config: Some(install_config).into(),
        ..Default::default()
    }
}

#[put("/v1/api/ota/put/{build_data}")]
async fn add_build_to_mapping(build_info: web::Path<String>) -> impl Responder {
    let build_meta = serde_json::de::from_str::<BuildMetadata>(build_info.as_str());
    if let Ok(metadata) = build_meta {
        let mut guard = BUILD_MAPPING.lock();
        let device = metadata.device.clone();
        if guard.contains_key(&device) {
            // we only want to track one build per device.
            guard.remove(&device);
        }
        guard.insert(metadata.device.clone(), metadata);
        "Success"
    } else {
        "Failure"
    }
}

#[get("/v1/api/ota/get/{metadata_bytes}")]
async fn handle_ota_request(metadata: web::Path<Vec<u8>>) -> impl Responder {
    let metadata = ota_proto::ota_metadata::DeviceState::parse_from_bytes(&metadata);
    // todo: pass out.
    let unwrapped_meta = metadata.map_err(|_| StatusCode::BAD_REQUEST).unwrap();
    for device in &(unwrapped_meta).device {
        if SUPPORTED_DEVICES.contains(&(device.as_str())) {
            let response = build_response_proto(device);
            return response.write_to_bytes().unwrap();
        }
    }
    vec![]
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .route("/hello", web::get().to(|| async { "Hello World!" }))
            .service(handle_ota_request)
    })
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}

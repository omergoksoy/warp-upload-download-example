use bytes:: Buf;
use futures::{TryStreamExt, StreamExt};
use tokio::task;
use std::{convert::Infallible, fs};
use warp::{
    http::StatusCode,
    Filter, Rejection, Reply,
};

#[tokio::main]
async fn main() {
    let upload_route = warp::path("upload")
        .and(warp::post())
        .and(warp::multipart::form().max_length(20_000_000))
        .and_then(upload);
    let download_route = warp::path("files").and(warp::fs::dir("./files/"));

    let router = upload_route.or(download_route).recover(handle_rejection);
    println!("Server started at localhost:1234");
    warp::serve(router).run(([0, 0, 0, 0], 1234)).await;
}

pub async fn upload(form: warp::multipart::FormData) -> Result<impl Reply, Rejection> {
    task::spawn(async move {
        let mut parts = form.into_stream();

        while let Ok(p) = parts.next().await.unwrap() {
            let filename = p.filename().unwrap_or("photo.png");
            let filepath = format!("uploads_test/{}", filename);
            fs::create_dir_all("uploads_test").unwrap();
            save_part_to_file(&filepath, p).await.expect("save error");
        }
    });

    Ok("Upload successful!")
}

async fn save_part_to_file(path: &str, part: warp::multipart::Part) -> Result<(), std::io::Error> {
    let data = part
        .stream()
        .try_fold(Vec::new(), |mut acc, buf| async move {
            acc.extend_from_slice(buf.chunk());
            Ok(acc)
        })
        .await.expect("folding error");
    std::fs::write(path, data)
}

async fn handle_rejection(err: Rejection) -> std::result::Result<impl Reply, Infallible> {
    let (code, message) = if err.is_not_found() {
        (StatusCode::NOT_FOUND, "Not Found".to_string())
    } else if err.find::<warp::reject::PayloadTooLarge>().is_some() {
        (StatusCode::BAD_REQUEST, "Payload too large".to_string())
    } else {
        eprintln!("unhandled error: {:?}", err);
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Internal Server Error".to_string(),
        )
    };

    Ok(warp::reply::with_status(message, code))
}

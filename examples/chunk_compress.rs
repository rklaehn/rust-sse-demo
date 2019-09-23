use flate2::write::DeflateEncoder;
use flate2::Compression;
use futures01::*;
use hyper::Body;
use std::fmt::Display;
use std::io::Write;
use std::io::{stdin, Read};
use std::time::{Duration, Instant};
use tokio::timer::Interval;
use warp::http::header::{HeaderValue, CONTENT_ENCODING, CONTENT_TYPE, TRANSFER_ENCODING};
use warp::reply::Response;
use warp::Filter;
use warp::Reply;

pub struct ChunkedReply<S: Stream>
where
    S::Item: Display,
{
    items: S,
}
impl<S: Stream> ChunkedReply<S>
where
    S::Item: Display,
{
    pub fn new(items: S) -> ChunkedReply<S> {
        ChunkedReply { items }
    }
}
impl<S: Stream + Send + 'static> Reply for ChunkedReply<S>
where
    S::Item: Display,
    S::Error: Send + Sync + std::error::Error,
{
    #[inline]
    fn into_response(self) -> Response {
        let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
        let keepalive = Interval::new(Instant::now(), Duration::from_secs(30))
            .map(|_instant| "\n".to_owned())
            .map_err(|e| panic!("interval errored; err={:?}", e));
        let es_with_keepalive = keepalive
            .select(self.items.map(|x| format!("{}\n", x)))
            .map(move |text| {
                encoder.write_all(text.as_bytes());
                encoder.flush();
                let mut bytes: &mut Vec<u8> = encoder.get_mut();
                let result = bytes.clone();
                bytes.clear();
                result
            });
        let mut res = Response::new(Body::wrap_stream(es_with_keepalive));
        res.headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("text/plain"));
        res.headers_mut()
            .insert(CONTENT_ENCODING, HeaderValue::from_static("deflate"));
        res.headers_mut()
            .insert(TRANSFER_ENCODING, HeaderValue::from_static("chunked"));
        res
    }
}

fn main() {
    let compression_header = warp::header::optional::<String>("Accept-Encoding");
    let app = warp::path("push-notifications")
        .and(compression_header)
        .and_then(|h: Option<String>| {
            if let Some(text) = h {
                let parts = text
                    .as_str()
                    .split(",")
                    .map(|x| x.trim())
                    .collect::<Vec<&str>>();
                println!("{:?}", parts);
                if parts.contains(&"deflate") {
                    Ok(ChunkedReply::new(
                        tokio::timer::Interval::new_interval(Duration::from_secs(1)).map(|_| "OK"),
                    ))
                } else {
                    Err(warp::reject::custom("unsupported encodings"))
                }
            } else {
                Err(warp::reject::custom("missing Accept-Encoding header"))
            }
        });

    let mut rt = tokio::runtime::Runtime::new().expect("could not start tokio runtime");
    let task = warp::serve(app).bind(([127, 0, 0, 1], 4244));
    rt.spawn(task);
    stdin().read(&mut [0]).unwrap();
    rt.shutdown_now();
}

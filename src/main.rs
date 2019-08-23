use futures01::stream::iter_ok;
use std::io::{stdin, Read};
use std::time::Duration;
use warp::{sse::ServerSentEvent, Filter};

fn main() {
    let app = warp::path("push-notifications")
        .and(warp::sse())
        .map(|sse: warp::sse::Sse| {
            let events = iter_ok::<_, ::std::io::Error>(vec![
                warp::sse::data("unnamed event").into_a(),
                (warp::sse::event("chat"), warp::sse::data("chat message"))
                    .into_a()
                    .into_b(),
                (
                    warp::sse::id(13),
                    warp::sse::event("chat"),
                    warp::sse::data("other chat message\nwith next line"),
                    warp::sse::retry(Duration::from_millis(5000)),
                )
                    .into_b()
                    .into_b(),
            ]);
            sse.reply(warp::sse::keep_alive().stream(events))
        });

    let mut rt = tokio::runtime::Runtime::new().expect("could not start tokio runtime");
    let (addr, task) = warp::serve(app).bind_ephemeral(([127, 0, 0, 1], 0));
    rt.spawn(task);
    println!("{:?}", addr);
    stdin().read(&mut [0]).unwrap();
    rt.shutdown_now();
}

use tokio;
use super::traits::{VFS, VPath};
use futures_core::Stream;
use std::io::Error;

enum Msg<P, F> {
    File(P, F),
    Dir(P),
    //Err(io::Error),
}

pub async fn copy<S, P, D: ?Sized>(source: S, dest: D)
where
    S: Stream<Item = Result<P, Error>> + Send,
    P: VPath,
    D: VPath + Send + Sync,
    // <D as VFS>::Path: VPath,
{

    tokio::spawn(async move {



    });

    crossbeam::scope(|scope| {
        let (sx, rx) = bounded(10);
        scope.spawn(move |_| {
            for p in source {
                let meta = match p.metadata() {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let msg = if meta.is_dir() {
                    Msg::Dir(p)
                } else if meta.is_file() {
                    if let Some(parent) = p.parent() {
                        sx.send(Msg::Dir(parent)).unwrap();
                    }
                    let file = p.open(OpenOptions::new().read(true)).unwrap();
                    Msg::File(p, file)
                } else {
                    continue;
                };

                sx.send(msg).unwrap();
            }
        });
        scope.spawn(move |_| loop {
            let mut msg = match rx.recv() {
                Ok(m) => m,
                Err(_) => return,
            };

            let ret = match &mut msg {
                Msg::Dir(path) => {
                    let path = dest.path(&path.to_string());
                    if path.exists() {
                        continue;
                    }
                    path.mkdir()
                }
                Msg::File(path, reader) => {
                    let path = dest.path(&path.to_string());
                    let mut file = path.open(OpenOptions::new().create(true)).unwrap();
                    io::copy(reader, &mut file).map(|_| ())
                }
            };
            if ret.is_err() {}
        });
    })
    .unwrap();
}

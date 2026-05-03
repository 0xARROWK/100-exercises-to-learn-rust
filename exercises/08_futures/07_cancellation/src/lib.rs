// TODO: fix the `assert_eq` at the end of the tests.
//  Do you understand why that's the resulting output?
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::net::TcpListener;

// In tests, run is spawned as a task.
// We pass 4 msgs through 4 TCP connections, using a timeout of 20ms
pub async fn run(listener: TcpListener, n_messages: usize, timeout: Duration) -> Vec<u8> {
    let mut buffer = Vec::new();
    for _ in 0..n_messages {
        let (mut stream, _) = listener.accept().await.unwrap();
        let _ = tokio::time::timeout(timeout, async {
            // read_to_end yield control back to runtime as soon as there is no more bytes to read.
            // When the first half of the message is sent, its fully read into the buffer.
            // Then we wait 40ms in tests, in order to exceed the timeout, so this task is cancelled.
            // That's why second half is never read.
            // Finally we enter the next loop iteration and repeat the process for the next msgs
            stream.read_to_end(&mut buffer).await.unwrap();
        })
        .await;
    }
    buffer
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncWriteExt;

    #[tokio::test]
    async fn ping() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let messages = vec!["hello", "from", "this", "task"];
        let timeout = Duration::from_millis(20);
        let handle = tokio::spawn(run(listener, messages.len(), timeout.clone()));

        for message in messages {
            let mut socket = tokio::net::TcpStream::connect(addr).await.unwrap();
            let (_, mut writer) = socket.split();

            let (beginning, end) = message.split_at(message.len() / 2);

            // Send first half
            writer.write_all(beginning.as_bytes()).await.unwrap();
            tokio::time::sleep(timeout * 2).await;
            writer.write_all(end.as_bytes()).await.unwrap();

            // Close the write side of the socket
            let _ = writer.shutdown().await;
        }

        let buffered = handle.await.unwrap();
        let buffered = std::str::from_utf8(&buffered).unwrap();
        assert_eq!(buffered, "hefrthta");
    }
}

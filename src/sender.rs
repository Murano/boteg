use failure::Fallible;
use hyper::{client::HttpConnector, Body, Client, Method, Request, Uri};
use hyper_tls::HttpsConnector;

pub struct Sender {
    uri: Uri,
    client: Client<HttpsConnector<HttpConnector>>,
}

impl Sender {
    pub fn new(uri: Uri) -> Self {
        let https = HttpsConnector::new();
        Self {
            uri,
            client: Client::builder().build::<_, hyper::Body>(https),
        }
    }

    pub async fn send_message(&self, body: Body) -> Fallible<()> {
        let req = Request::builder()
            .method(Method::POST)
            .uri(&self.uri)
            .header("content-type", "application/json")
            .body(body)?;
        let _ = self.client.request(req).await?;
        Ok(())
    }
}

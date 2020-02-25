use failure::Fallible;
use hyper::{Body, Uri, Client, Method, Request};
use hyper_tls::HttpsConnector;
use hyper::client::HttpConnector;

pub struct Sender {
    uri: Uri,
    client: Client<HttpsConnector<HttpConnector>>
}

impl Sender {
    
    pub fn new(uri: Uri) -> Self {
        let https = HttpsConnector::new();
        Self {
            uri,
            client: Client::builder()
                .build::<_, hyper::Body>(https)
        }
    }
    
    pub async fn send_message(&self, body: Body) -> Fallible<()>{
        let req = Request::builder()
            .method(Method::POST)
            .uri(&self.uri)
            .header("content-type", "application/json")
            .body(body)?;
        let res = self.client.request(req).await?;
        Ok(())
    }
}
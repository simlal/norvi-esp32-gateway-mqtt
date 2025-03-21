use log::{error, info};

use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::Stack;
use reqwless::client::{HttpClient, TlsConfig};
use reqwless::request::{Method, RequestBuilder};

const BUFFER_SIZE: usize = 0x1000;

pub async fn make_get_request(stack: Stack<'_>, tls_seed: u64, url: &str) {
    let mut rx_buffer = [0; BUFFER_SIZE];
    let mut tx_buffer = [0; BUFFER_SIZE];
    let dns = DnsSocket::new(stack);
    let tcp_state = TcpClientState::<1, BUFFER_SIZE, BUFFER_SIZE>::new();
    let tcp = TcpClient::new(stack, &tcp_state);

    let tls = TlsConfig::new(
        tls_seed,
        &mut rx_buffer,
        &mut tx_buffer,
        reqwless::client::TlsVerify::None,
    );

    let mut client = HttpClient::new_with_tls(&tcp, &dns, tls);
    let mut buffer = [0u8; BUFFER_SIZE];

    // TODO: PASS IN PARAMS TO CONSTRUCT URL ?

    // Create the request
    let http_req = match client.request(Method::GET, url).await {
        Ok(req) => req,
        Err(e) => {
            error!("Could not create request from {}. Error: {:?}", url, e);
            return;
        }
    };

    // TODO: PASS IN  HEADERS FROM FUNC SIG
    let mut http_req_with_headers = http_req
        .content_type(reqwless::headers::ContentType::TextPlain)
        .headers(&[("x-api-key", "MYKEY")]);

    info!("GET Request to '{}' created, attempting to send...", url);
    let http_res = match http_req_with_headers.send(&mut buffer).await {
        Ok(res) => {
            info!("Response statusCode: {:?}", res.status);
            res
        }
        Err(e) => {
            error!("Could not perform http request. Error: {:?}", e);
            return;
        }
    };

    info!("Got response, attempting to print content...");
    let res = match http_res.body().read_to_end().await {
        Ok(data) => data,
        Err(e) => {
            error!("Failed to read response body: {:?}", e);
            return;
        }
    };

    match core::str::from_utf8(res) {
        Ok(content) => info!("{}", content),
        Err(e) => error!("Response wasn't valid UTF-8: {:?}", e),
    }
}

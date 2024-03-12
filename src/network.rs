pub struct Header {
    name: String,
    value: String,
}
pub struct Headers {
    headers: Vec<Header>,
}

impl Headers {
    pub fn contains(&self, name: String) -> bool {
        self.headers.iter().any(|h| h.name == name)
    }
}
#[rocket::async_trait]
impl<'r> rocket::request::FromRequest<'r> for Headers {
    type Error = ();
    async fn from_request(
        request: &'r rocket::request::Request<'_>,
    ) -> rocket::request::Outcome<Self, Self::Error> {
        let headers = request
            .headers()
            .iter()
            .map(|header| Header {
                name: header.name().to_string(),
                value: header.value().to_string(),
            })
            .collect();
        rocket::request::Outcome::Success(Headers { headers })
    }
}

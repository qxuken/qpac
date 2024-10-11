use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    body::Body,
    http::{Response, StatusCode},
    response::IntoResponse,
};
use ring::constant_time::verify_slices_are_equal;
use tower_http::validate_request::{ValidateRequest, ValidateRequestHeaderLayer};
use tracing::info;

use crate::error::AppError;

pub fn use_auth_layer(token: String) -> ValidateRequestHeaderLayer<AuthTokenValidator> {
    ValidateRequestHeaderLayer::custom(AuthTokenValidator::new(token))
}

#[derive(Clone)]
pub enum AuthTokenValidator {
    Simple(SimpleAuthTokenValidator),
    Argon2(Argon2AuthTokenValidator),
}

impl AuthTokenValidator {
    fn new(token: String) -> Self {
        if token.starts_with("$argon2") {
            AuthTokenValidator::Argon2(Argon2AuthTokenValidator::new(token))
        } else {
            info!("Token is not secure, consider using argon2 phc format");
            AuthTokenValidator::Simple(SimpleAuthTokenValidator::new(token.as_bytes().to_vec()))
        }
    }
}

impl<B> ValidateRequest<B> for AuthTokenValidator {
    type ResponseBody = Body;

    fn validate(
        &mut self,
        request: &mut axum::http::Request<B>,
    ) -> std::result::Result<(), Response<Self::ResponseBody>> {
        match self {
            Self::Simple(v) => v.validate(request),
            Self::Argon2(v) => v.validate(request),
        }
    }
}

#[derive(Clone)]
pub struct SimpleAuthTokenValidator {
    token: Vec<u8>,
}

impl SimpleAuthTokenValidator {
    pub fn new(token: Vec<u8>) -> Self {
        Self { token }
    }
}

impl<B> ValidateRequest<B> for SimpleAuthTokenValidator {
    type ResponseBody = Body;

    fn validate(
        &mut self,
        request: &mut axum::http::Request<B>,
    ) -> std::result::Result<(), Response<Self::ResponseBody>> {
        let raw_token = extract_token(request)?;

        verify_slices_are_equal(raw_token.as_bytes(), &self.token)
            .map_err(|_| response_unathorized("Unathorized"))
    }
}

#[derive(Clone)]
pub struct Argon2AuthTokenValidator {
    token: String,
}

impl Argon2AuthTokenValidator {
    pub fn new(token: String) -> Self {
        Self { token }
    }
}

impl<B> ValidateRequest<B> for Argon2AuthTokenValidator {
    type ResponseBody = Body;

    fn validate(
        &mut self,
        request: &mut axum::http::Request<B>,
    ) -> std::result::Result<(), Response<Self::ResponseBody>> {
        let Ok(hash) = PasswordHash::new(&self.token) else {
            return Err(AppError::Other("Server error".to_string()).into_response());
        };
        let raw_token = extract_token(request)?;
        let argon = Argon2::default();

        argon
            .verify_password(raw_token.as_bytes(), &hash)
            .map_err(|_| response_unathorized("Unathorized"))
    }
}

fn extract_token<B>(
    request: &mut axum::http::Request<B>,
) -> std::result::Result<String, Response<Body>> {
    let Some(auth_header) = request.headers().get("Authorization") else {
        return Err(response_unathorized("Missing auth token"));
    };
    let Ok(full_token_str) = auth_header.to_str() else {
        return Err(response_unathorized("Bad token"));
    };

    full_token_str
        .trim()
        .strip_prefix("Bearer ")
        .map(String::from)
        .ok_or_else(|| response_unathorized("Token should be Bearer"))
}

fn response_unathorized(msg: impl Into<String>) -> Response<Body> {
    (StatusCode::UNAUTHORIZED, msg.into()).into_response()
}

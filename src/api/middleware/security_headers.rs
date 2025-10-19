// src/api/middleware/security_headers.rs

use axum::{
    extract::Request,
    http::{header, HeaderValue},
    middleware::Next,
    response::Response,
};

/// Security headers configuration
#[derive(Debug, Clone)]
pub struct SecurityHeadersConfig {
    /// Enable Content Security Policy
    pub enable_csp: bool,
    /// CSP directives
    pub csp_directives: String,
    /// Enable HTTP Strict Transport Security
    pub enable_hsts: bool,
    /// HSTS max age in seconds
    pub hsts_max_age: u32,
    /// Include subdomains in HSTS
    pub hsts_include_subdomains: bool,
    /// Enable HSTS preload
    pub hsts_preload: bool,
    /// Enable X-Frame-Options
    pub enable_frame_options: bool,
    /// X-Frame-Options value
    pub frame_options: FrameOptions,
    /// Enable X-Content-Type-Options
    pub enable_content_type_options: bool,
    /// Enable X-XSS-Protection
    pub enable_xss_protection: bool,
    /// Enable Referrer-Policy
    pub enable_referrer_policy: bool,
    /// Referrer policy value
    pub referrer_policy: ReferrerPolicy,
    /// Enable Permissions-Policy
    pub enable_permissions_policy: bool,
    /// Permissions policy directives
    pub permissions_policy: String,
}

/// X-Frame-Options values
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameOptions {
    /// DENY - page cannot be displayed in a frame
    Deny,
    /// SAMEORIGIN - page can only be displayed in a frame on the same origin
    SameOrigin,
    /// ALLOW-FROM uri - page can only be displayed in a frame on the specified origin
    AllowFrom(String),
}

impl FrameOptions {
    fn as_str(&self) -> String {
        match self {
            FrameOptions::Deny => "DENY".to_string(),
            FrameOptions::SameOrigin => "SAMEORIGIN".to_string(),
            FrameOptions::AllowFrom(uri) => format!("ALLOW-FROM {}", uri),
        }
    }
}

/// Referrer-Policy values
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferrerPolicy {
    /// No referrer information is sent
    NoReferrer,
    /// No referrer when downgrading from HTTPS to HTTP
    NoReferrerWhenDowngrade,
    /// Only send origin as referrer
    Origin,
    /// Send origin when cross-origin, full URL when same-origin
    OriginWhenCrossOrigin,
    /// Only send referrer for same-origin requests
    SameOrigin,
    /// Send origin for same-origin, no referrer for cross-origin
    StrictOrigin,
    /// Send full URL for same-origin, origin for cross-origin HTTPS, no referrer for HTTP
    StrictOriginWhenCrossOrigin,
    /// Always send full URL
    UnsafeUrl,
}

impl ReferrerPolicy {
    fn as_str(&self) -> &str {
        match self {
            ReferrerPolicy::NoReferrer => "no-referrer",
            ReferrerPolicy::NoReferrerWhenDowngrade => "no-referrer-when-downgrade",
            ReferrerPolicy::Origin => "origin",
            ReferrerPolicy::OriginWhenCrossOrigin => "origin-when-cross-origin",
            ReferrerPolicy::SameOrigin => "same-origin",
            ReferrerPolicy::StrictOrigin => "strict-origin",
            ReferrerPolicy::StrictOriginWhenCrossOrigin => "strict-origin-when-cross-origin",
            ReferrerPolicy::UnsafeUrl => "unsafe-url",
        }
    }
}

impl Default for SecurityHeadersConfig {
    fn default() -> Self {
        Self {
            enable_csp: true,
            csp_directives: "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'".to_string(),
            enable_hsts: true,
            hsts_max_age: 31536000, // 1 year
            hsts_include_subdomains: true,
            hsts_preload: false,
            enable_frame_options: true,
            frame_options: FrameOptions::Deny,
            enable_content_type_options: true,
            enable_xss_protection: true,
            enable_referrer_policy: true,
            referrer_policy: ReferrerPolicy::StrictOriginWhenCrossOrigin,
            enable_permissions_policy: true,
            permissions_policy: "geolocation=(), microphone=(), camera=()".to_string(),
        }
    }
}

impl SecurityHeadersConfig {
    /// Creates a security headers configuration from environment variables
    pub fn from_env() -> Self {
        let enable_csp = std::env::var("XZEPR__SECURITY__HEADERS__ENABLE_CSP")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(true);

        let csp_directives = std::env::var("XZEPR__SECURITY__HEADERS__CSP_DIRECTIVES")
            .unwrap_or_else(|_| Self::default().csp_directives);

        let enable_hsts = std::env::var("XZEPR__SECURITY__HEADERS__ENABLE_HSTS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(true);

        let hsts_max_age = std::env::var("XZEPR__SECURITY__HEADERS__HSTS_MAX_AGE")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(31536000);

        let hsts_include_subdomains =
            std::env::var("XZEPR__SECURITY__HEADERS__HSTS_INCLUDE_SUBDOMAINS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(true);

        let hsts_preload = std::env::var("XZEPR__SECURITY__HEADERS__HSTS_PRELOAD")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(false);

        Self {
            enable_csp,
            csp_directives,
            enable_hsts,
            hsts_max_age,
            hsts_include_subdomains,
            hsts_preload,
            ..Default::default()
        }
    }

    /// Creates a permissive configuration for development
    pub fn permissive() -> Self {
        Self {
            enable_csp: true,
            csp_directives: "default-src *; script-src * 'unsafe-inline' 'unsafe-eval'; style-src * 'unsafe-inline'".to_string(),
            enable_hsts: false,
            hsts_max_age: 0,
            hsts_include_subdomains: false,
            hsts_preload: false,
            enable_frame_options: false,
            frame_options: FrameOptions::SameOrigin,
            enable_content_type_options: true,
            enable_xss_protection: false,
            enable_referrer_policy: false,
            referrer_policy: ReferrerPolicy::NoReferrerWhenDowngrade,
            enable_permissions_policy: false,
            permissions_policy: String::new(),
        }
    }

    /// Creates a strict production configuration
    pub fn production() -> Self {
        Self {
            enable_csp: true,
            csp_directives: "default-src 'self'; script-src 'self'; style-src 'self'; img-src 'self' https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'; base-uri 'self'; form-action 'self'".to_string(),
            enable_hsts: true,
            hsts_max_age: 63072000, // 2 years
            hsts_include_subdomains: true,
            hsts_preload: true,
            enable_frame_options: true,
            frame_options: FrameOptions::Deny,
            enable_content_type_options: true,
            enable_xss_protection: true,
            enable_referrer_policy: true,
            referrer_policy: ReferrerPolicy::StrictOriginWhenCrossOrigin,
            enable_permissions_policy: true,
            permissions_policy: "geolocation=(), microphone=(), camera=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=()".to_string(),
        }
    }

    /// Creates a configuration suitable for API-only applications
    pub fn api_only() -> Self {
        Self {
            enable_csp: false,
            csp_directives: String::new(),
            enable_hsts: true,
            hsts_max_age: 31536000,
            hsts_include_subdomains: true,
            hsts_preload: false,
            enable_frame_options: true,
            frame_options: FrameOptions::Deny,
            enable_content_type_options: true,
            enable_xss_protection: false,
            enable_referrer_policy: true,
            referrer_policy: ReferrerPolicy::NoReferrer,
            enable_permissions_policy: false,
            permissions_policy: String::new(),
        }
    }
}

/// Security headers middleware
///
/// Adds various security headers to responses to protect against common web vulnerabilities
///
/// # Security Headers Added
///
/// - **Content-Security-Policy**: Controls resources the browser can load
/// - **Strict-Transport-Security**: Forces HTTPS connections
/// - **X-Frame-Options**: Prevents clickjacking attacks
/// - **X-Content-Type-Options**: Prevents MIME type sniffing
/// - **X-XSS-Protection**: Enables browser XSS filtering (legacy)
/// - **Referrer-Policy**: Controls referrer information
/// - **Permissions-Policy**: Controls browser features
///
/// # Example
///
/// ```ignore
/// use axum::Router;
/// use xzepr::api::middleware::security_headers::security_headers_middleware;
///
/// let app = Router::new()
///     .layer(axum::middleware::from_fn(security_headers_middleware));
/// ```
pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let config = SecurityHeadersConfig::default();
    security_headers_middleware_with_config(config, request, next).await
}

/// Security headers middleware with custom configuration
pub async fn security_headers_middleware_with_config(
    config: SecurityHeadersConfig,
    request: Request,
    next: Next,
) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Content-Security-Policy
    if config.enable_csp && !config.csp_directives.is_empty() {
        if let Ok(value) = HeaderValue::from_str(&config.csp_directives) {
            headers.insert(header::CONTENT_SECURITY_POLICY, value);
        }
    }

    // Strict-Transport-Security
    if config.enable_hsts {
        let mut hsts_value = format!("max-age={}", config.hsts_max_age);
        if config.hsts_include_subdomains {
            hsts_value.push_str("; includeSubDomains");
        }
        if config.hsts_preload {
            hsts_value.push_str("; preload");
        }
        if let Ok(value) = HeaderValue::from_str(&hsts_value) {
            headers.insert(header::STRICT_TRANSPORT_SECURITY, value);
        }
    }

    // X-Frame-Options
    if config.enable_frame_options {
        let frame_options_value = config.frame_options.as_str();
        if let Ok(value) = HeaderValue::from_str(&frame_options_value) {
            headers.insert(header::X_FRAME_OPTIONS, value);
        }
    }

    // X-Content-Type-Options
    if config.enable_content_type_options {
        headers.insert(
            header::X_CONTENT_TYPE_OPTIONS,
            HeaderValue::from_static("nosniff"),
        );
    }

    // X-XSS-Protection (legacy, but still useful for older browsers)
    if config.enable_xss_protection {
        headers.insert(
            header::HeaderName::from_static("x-xss-protection"),
            HeaderValue::from_static("1; mode=block"),
        );
    }

    // Referrer-Policy
    if config.enable_referrer_policy {
        let referrer_value = config.referrer_policy.as_str();
        if let Ok(value) = HeaderValue::from_str(referrer_value) {
            headers.insert(header::REFERRER_POLICY, value);
        }
    }

    // Permissions-Policy
    if config.enable_permissions_policy && !config.permissions_policy.is_empty() {
        if let Ok(value) = HeaderValue::from_str(&config.permissions_policy) {
            headers.insert(header::HeaderName::from_static("permissions-policy"), value);
        }
    }

    // Additional security headers
    headers.insert(
        header::HeaderName::from_static("x-permitted-cross-domain-policies"),
        HeaderValue::from_static("none"),
    );

    headers.insert(
        header::HeaderName::from_static("x-download-options"),
        HeaderValue::from_static("noopen"),
    );

    // Remove server identification
    headers.remove(header::SERVER);

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        response::IntoResponse,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn test_handler() -> impl IntoResponse {
        (StatusCode::OK, "test response")
    }

    #[tokio::test]
    async fn test_default_security_headers() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(middleware::from_fn(security_headers_middleware));

        let request = Request::builder().uri("/").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        let headers = response.headers();
        assert!(headers.contains_key(header::CONTENT_SECURITY_POLICY));
        assert!(headers.contains_key(header::STRICT_TRANSPORT_SECURITY));
        assert!(headers.contains_key(header::X_FRAME_OPTIONS));
        assert!(headers.contains_key(header::X_CONTENT_TYPE_OPTIONS));
        assert!(headers.contains_key(header::REFERRER_POLICY));
        assert!(!headers.contains_key(header::SERVER));
    }

    #[tokio::test]
    async fn test_production_config() {
        let config = SecurityHeadersConfig::production();
        assert!(config.enable_hsts);
        assert_eq!(config.hsts_max_age, 63072000);
        assert!(config.hsts_preload);
        assert_eq!(config.frame_options, FrameOptions::Deny);
    }

    #[tokio::test]
    async fn test_permissive_config() {
        let config = SecurityHeadersConfig::permissive();
        assert!(!config.enable_hsts);
        assert!(!config.enable_frame_options);
        assert!(config.csp_directives.contains("*"));
    }

    #[tokio::test]
    async fn test_api_only_config() {
        let config = SecurityHeadersConfig::api_only();
        assert!(!config.enable_csp);
        assert!(config.enable_hsts);
        assert!(!config.enable_permissions_policy);
    }

    #[test]
    fn test_frame_options_values() {
        assert_eq!(FrameOptions::Deny.as_str(), "DENY");
        assert_eq!(FrameOptions::SameOrigin.as_str(), "SAMEORIGIN");
        assert_eq!(
            FrameOptions::AllowFrom("https://example.com".to_string()).as_str(),
            "ALLOW-FROM https://example.com"
        );
    }

    #[test]
    fn test_referrer_policy_values() {
        assert_eq!(ReferrerPolicy::NoReferrer.as_str(), "no-referrer");
        assert_eq!(
            ReferrerPolicy::NoReferrerWhenDowngrade.as_str(),
            "no-referrer-when-downgrade"
        );
        assert_eq!(ReferrerPolicy::Origin.as_str(), "origin");
        assert_eq!(
            ReferrerPolicy::OriginWhenCrossOrigin.as_str(),
            "origin-when-cross-origin"
        );
        assert_eq!(ReferrerPolicy::SameOrigin.as_str(), "same-origin");
        assert_eq!(ReferrerPolicy::StrictOrigin.as_str(), "strict-origin");
        assert_eq!(
            ReferrerPolicy::StrictOriginWhenCrossOrigin.as_str(),
            "strict-origin-when-cross-origin"
        );
        assert_eq!(ReferrerPolicy::UnsafeUrl.as_str(), "unsafe-url");
    }

    #[test]
    fn test_hsts_header_format() {
        let config = SecurityHeadersConfig {
            enable_hsts: true,
            hsts_max_age: 31536000,
            hsts_include_subdomains: true,
            hsts_preload: true,
            ..Default::default()
        };

        let expected = "max-age=31536000; includeSubDomains; preload";
        let mut actual = format!("max-age={}", config.hsts_max_age);
        if config.hsts_include_subdomains {
            actual.push_str("; includeSubDomains");
        }
        if config.hsts_preload {
            actual.push_str("; preload");
        }

        assert_eq!(actual, expected);
    }

    #[tokio::test]
    async fn test_csp_header_custom_directives() {
        let custom_csp = "default-src 'self'; script-src 'self' https://cdn.example.com";
        let config = SecurityHeadersConfig {
            enable_csp: true,
            csp_directives: custom_csp.to_string(),
            ..Default::default()
        };

        let app = Router::new()
            .route("/", get(test_handler))
            .layer(middleware::from_fn(move |req, next| {
                let config = config.clone();
                async move { security_headers_middleware_with_config(config, req, next).await }
            }));

        let request = Request::builder().uri("/").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();
        let csp = response
            .headers()
            .get(header::CONTENT_SECURITY_POLICY)
            .unwrap()
            .to_str()
            .unwrap();

        assert_eq!(csp, custom_csp);
    }

    #[tokio::test]
    async fn test_headers_removed() {
        let app = Router::new()
            .route("/", get(test_handler))
            .layer(middleware::from_fn(security_headers_middleware));

        let request = Request::builder().uri("/").body(Body::empty()).unwrap();

        let response = app.oneshot(request).await.unwrap();

        // Server header should be removed
        assert!(!response.headers().contains_key(header::SERVER));
    }

    #[test]
    fn test_config_from_default() {
        let config = SecurityHeadersConfig::default();
        assert!(config.enable_csp);
        assert!(config.enable_hsts);
        assert_eq!(config.hsts_max_age, 31536000);
        assert!(config.enable_frame_options);
        assert!(config.enable_content_type_options);
    }
}

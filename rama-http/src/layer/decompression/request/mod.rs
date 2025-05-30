pub(super) mod layer;
pub(super) mod service;

#[cfg(test)]
mod tests {
    use super::service::RequestDecompression;

    use crate::dep::http_body_util::BodyExt;
    use crate::layer::decompression::DecompressionBody;
    use crate::{Body, Request, Response, StatusCode, header};
    use rama_core::service::service_fn;
    use rama_core::{Context, Service};

    use flate2::{Compression, write::GzEncoder};
    use std::{convert::Infallible, io::Write};

    #[tokio::test]
    async fn decompress_accepted_encoding() {
        let req = request_gzip();
        let svc = RequestDecompression::new(service_fn(assert_request_is_decompressed));
        let _ = svc.serve(Context::default(), req).await.unwrap();
    }

    #[tokio::test]
    async fn support_unencoded_body() {
        let req = Request::builder().body(Body::from("Hello?")).unwrap();
        let svc = RequestDecompression::new(service_fn(assert_request_is_decompressed));
        let _ = svc.serve(Context::default(), req).await.unwrap();
    }

    #[tokio::test]
    async fn unaccepted_content_encoding_returns_unsupported_media_type() {
        let req = request_gzip();
        let svc = RequestDecompression::new(service_fn(should_not_be_called)).gzip(false);
        let res = svc.serve(Context::default(), req).await.unwrap();
        assert_eq!(StatusCode::UNSUPPORTED_MEDIA_TYPE, res.status());
    }

    #[tokio::test]
    async fn pass_through_unsupported_encoding_when_enabled() {
        let req = request_gzip();
        let svc = RequestDecompression::new(service_fn(assert_request_is_passed_through))
            .pass_through_unaccepted(true)
            .gzip(false);
        let _ = svc.serve(Context::default(), req).await.unwrap();
    }

    async fn assert_request_is_decompressed(
        req: Request<DecompressionBody<Body>>,
    ) -> Result<Response<Body>, Infallible> {
        let (parts, mut body) = req.into_parts();
        let body = read_body(&mut body).await;

        assert_eq!(body, b"Hello?");
        assert!(!parts.headers.contains_key(header::CONTENT_ENCODING));

        Ok(Response::new(Body::from("Hello, World!")))
    }

    async fn assert_request_is_passed_through(
        req: Request<DecompressionBody<Body>>,
    ) -> Result<Response<Body>, Infallible> {
        let (parts, mut body) = req.into_parts();
        let body = read_body(&mut body).await;

        assert_ne!(body, b"Hello?");
        assert!(parts.headers.contains_key(header::CONTENT_ENCODING));

        Ok(Response::new(Body::empty()))
    }

    async fn should_not_be_called(
        _: Request<DecompressionBody<Body>>,
    ) -> Result<Response<Body>, Infallible> {
        panic!("Inner service should not be called");
    }

    fn request_gzip() -> Request<Body> {
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(b"Hello?").unwrap();
        let body = encoder.finish().unwrap();
        Request::builder()
            .header(header::CONTENT_ENCODING, "gzip")
            .body(Body::from(body))
            .unwrap()
    }

    async fn read_body(body: &mut DecompressionBody<Body>) -> Vec<u8> {
        body.collect().await.unwrap().to_bytes().to_vec()
    }
}

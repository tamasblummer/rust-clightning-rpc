// Rust JSON-RPC Library
// Written in 2015 by
//   Andrew Poelstra <apoelstra@wpsoftware.net>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the CC0 Public Domain Dedication
// along with this software.
// If not, see <http://creativecommons.org/publicdomain/zero/1.0/>.
//

//! Crate that provides an RPC binding from rust code to the c-lightning daemon
//!
//! This create provides both a high and a low-level interface.
//! Most likely, you'll want to use the high-level interface through `LightningRPC`, as this is
//! most convenient,
//! but it is also possible to construct Request and Response objects manually and
//! send them through the pipe.

#![crate_type = "lib"]
#![crate_type = "rlib"]
#![crate_type = "dylib"]
#![crate_name = "clightningrpc"]
// Coding conventions
#![deny(non_upper_case_globals)]
#![deny(non_camel_case_types)]
#![deny(non_snake_case)]
#![deny(unused_mut)]
#![warn(missing_docs)]

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate strason;

pub mod client;
pub mod common;
pub mod error;
pub mod lightningrpc;
pub mod requests;
pub mod responses;

use strason::Json;
// Re-export error type
pub use error::Error;
// Re-export high-level connection type
pub use lightningrpc::LightningRPC;

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
/// A JSONRPC request object
pub struct Request {
    /// The name of the RPC call
    pub method: String,
    /// Parameters to the RPC call
    pub params: Json,
    /// Identifier for this Request, which should appear in the response
    pub id: Json,
    /// jsonrpc field, MUST be "2.0"
    pub jsonrpc: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
/// A JSONRPC response object
pub struct Response {
    /// A result if there is one, or null
    pub result: Option<Json>,
    /// An error if there is one, or null
    pub error: Option<error::RpcError>,
    /// Identifier for this Request, which should match that of the request
    pub id: Json,
    /// jsonrpc field, MUST be "2.0"
    pub jsonrpc: Option<String>,
}

impl Response {
    /// Extract the result from a response
    pub fn result<T: serde::de::DeserializeOwned>(&self) -> Result<T, Error> {
        if let Some(ref e) = self.error {
            return Err(Error::Rpc(e.clone()));
        }
        match self.result {
            Some(ref res) => res.clone().into_deserialize().map_err(Error::Json),
            None => Err(Error::NoErrorOrResult),
        }
    }

    /// Extract the result from a response, consuming the response
    pub fn into_result<T: serde::de::DeserializeOwned>(self) -> Result<T, Error> {
        if let Some(e) = self.error {
            return Err(Error::Rpc(e));
        }

        match self.result {
            Some(ref res) => res.clone().into_deserialize().map_err(Error::Json),
            None => Err(Error::NoErrorOrResult),
        }
    }

    /// Return the RPC error, if there was one, but do not check the result
    pub fn check_error(self) -> Result<(), Error> {
        if let Some(e) = self.error {
            Err(Error::Rpc(e))
        } else {
            Ok(())
        }
    }

    /// Returns whether or not the `result` field is empty
    pub fn is_none(&self) -> bool {
        self.result.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::error::RpcError;
    use super::{Request, Response};
    use strason::Json;

    #[test]
    fn request_serialize_round_trip() {
        let original = Request {
            method: "test".to_owned(),
            params: From::from(vec![
                ("a".to_string(), From::from(())),
                ("b".to_string(), From::from(false)),
                ("c".to_string(), From::from(true)),
                ("d".to_string(), From::from("test2")),
            ]),
            id: From::from("69"),
            jsonrpc: Some(String::from("2.0")),
        };

        let ser = Json::from_serialize(&original).unwrap();
        let des = ser.into_deserialize().unwrap();

        assert_eq!(original, des);
    }

    #[test]
    fn response_serialize_round_trip() {
        let original_err = RpcError {
            code: -77,
            message: "test4".to_owned(),
            data: Some(From::from(true)),
        };

        let original = Response {
            result: Some(From::<Vec<Json>>::from(vec![
                From::from(()),
                From::from(false),
                From::from(true),
                From::from("test2"),
            ])),
            error: Some(original_err),
            id: From::from(101),
            jsonrpc: Some(String::from("2.0")),
        };

        let ser = Json::from_serialize(&original).unwrap();
        let des = ser.into_deserialize().unwrap();

        assert_eq!(original, des);
    }

    #[test]
    fn response_is_none() {
        let joanna = Response {
            result: Some(From::from(true)),
            error: None,
            id: From::from(81),
            jsonrpc: Some(String::from("2.0")),
        };

        let bill = Response {
            result: None,
            error: None,
            id: From::from(66),
            jsonrpc: Some(String::from("2.0")),
        };

        assert!(!joanna.is_none());
        assert!(bill.is_none());
    }

    #[test]
    fn response_extract() {
        let obj = vec!["Mary", "had", "a", "little", "lamb"];
        let response = Response {
            result: Some(Json::from_serialize(&obj).unwrap()),
            error: None,
            id: From::from(()),
            jsonrpc: Some(String::from("2.0")),
        };
        let recovered1: Vec<String> = response.result().unwrap();
        assert!(response.clone().check_error().is_ok());
        let recovered2: Vec<String> = response.into_result().unwrap();
        assert_eq!(obj, recovered1);
        assert_eq!(obj, recovered2);
    }
}

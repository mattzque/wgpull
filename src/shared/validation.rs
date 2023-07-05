use anyhow::Result;
use ipnet::IpNet;
use std::net::IpAddr;
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ValidationError {
    #[error("Value for {0} is empty or not provided")]
    EmptyValue(&'static str),
    #[error("Unexpected value for {0}: {1}")]
    InvalidFormat(&'static str, &'static str),
}

pub trait Validated {
    /// Validates itself, resulting in a `ValidationError` whenever a validation rule fails.
    fn validate(&self) -> Result<(), ValidationError>;
}

pub fn validate_hostname(name: &'static str, hostname: &str) -> Result<(), ValidationError> {
    if hostname.len() > 253 {
        return Err(ValidationError::InvalidFormat(
            name,
            "Hostname is longer than 253 characters",
        ));
    }

    for label in hostname.split('.') {
        let len = label.len();
        if len == 0 || len > 63 {
            return Err(ValidationError::InvalidFormat(
                name,
                "A label is empty or longer than 63 characters",
            ));
        }

        if !label.chars().all(|c| c.is_alphanumeric() || c == '-')
            || label.starts_with('-')
            || label.ends_with('-')
        {
            return Err(ValidationError::InvalidFormat(
                name,
                "Invalid character in label or label starts/ends with a hyphen",
            ));
        }
    }

    Ok(())
}

pub fn validate_ip(name: &'static str, ip: &str) -> Result<(), ValidationError> {
    if IpAddr::from_str(ip).is_err() {
        return Err(ValidationError::InvalidFormat(name, "Invalid IP address"));
    }

    Ok(())
}

pub fn validate_hostname_or_ip(
    name: &'static str,
    hostname_or_ip: &str,
) -> Result<(), ValidationError> {
    if hostname_or_ip.is_empty() {
        return Err(ValidationError::EmptyValue(name));
    }

    let hostname_result = validate_hostname(name, hostname_or_ip);
    let ip_result = validate_ip(name, hostname_or_ip);

    if hostname_result.is_err() && ip_result.is_err() {
        return hostname_result;
    }

    Ok(())
}

pub fn validate_cidr(name: &'static str, address: &str) -> Result<(), ValidationError> {
    if address.is_empty() {
        return Err(ValidationError::EmptyValue(name));
    }

    if IpNet::from_str(address).is_err() {
        return Err(ValidationError::InvalidFormat(
            name,
            "Invalid IP network CIDR",
        ));
    }

    Ok(())
}

pub fn validate_wg_key(name: &'static str, key: &str) -> Result<(), ValidationError> {
    if key.is_empty() {
        return Err(ValidationError::EmptyValue(name));
    }

    match base64::decode(key) {
        Ok(decoded) => {
            if decoded.len() == 32 {
                Ok(())
            } else {
                Err(ValidationError::InvalidFormat(
                    name,
                    "Invalid key length (32)",
                ))
            }
        }
        Err(_) => Err(ValidationError::InvalidFormat(name, "Invalid base64")),
    }
}

pub fn validate_interface_name(name: &'static str, interface: &str) -> Result<(), ValidationError> {
    if interface.is_empty() {
        return Err(ValidationError::EmptyValue(name));
    }

    if interface.len() > 15 {
        return Err(ValidationError::InvalidFormat(
            name,
            "Invalid interface length (16 max)",
        ));
    }

    if !interface.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err(ValidationError::InvalidFormat(
            name,
            "Interface name contains invalid characters",
        ));
    }

    Ok(())
}

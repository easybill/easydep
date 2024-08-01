/*
 * This file is part of easydep, licensed under the MIT License (MIT).
 *
 * Copyright (c) 2024 easybill GmbH
 *
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 *
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 *
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 */
use anyhow::bail;
use tonic::transport::Uri;

/// Parses and validates the given address to be a valid gRPC endpoint uri, returning an error if that is not the case.
/// This method does some additional checks that are not included in `Uri::try_from`.
///
/// # Arguments
/// * `address` - The address to parse and check to be a valid endpoint.
pub(crate) fn validate_grpc_endpoint_uri(address: &String) -> anyhow::Result<Uri> {
    match Uri::try_from(address) {
        Ok(uri) => {
            if uri.host().is_none() {
                bail!("invalid endpoint uri {}: host is missing", address)
            }
            if uri.scheme().is_none() {
                bail!("invalid endpoint uri {}: scheme is missing", address)
            }

            Ok(uri)
        }
        Err(err) => bail!("invalid uri provided {}: {}", address, err),
    }
}

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

use std::collections::HashSet;
use std::future::Future;

use anyhow::{anyhow, Context};
use futures::future;

use crate::config::TargetServer;

/// Executes the given callback function asynchronously for each of the given servers,
/// also providing the previously opened client connection.
///
/// # Arguments
/// * `servers` - The target servers to call the given callback for.
/// * `connection_opener` - The function to call to open a connection to the target server.
/// * `request_executor` - The function to call to execute the actual request for a target server.
///
/// # Returns
/// * `anyhow::Result<()>` - Either `Ok` when all tasks completed successfully or the first captured error.
pub(crate) async fn execute_for_servers<Con, FuncCo, FuncEx, FutCo, FutEx>(
    servers: HashSet<&TargetServer>,
    connection_opener: FuncCo,
    request_executor: FuncEx,
) -> anyhow::Result<()>
where
    FuncCo: Fn(TargetServer) -> FutCo + Clone + Send + 'static,
    FuncEx: Fn(TargetServer, Con) -> FutEx + Clone + Send + 'static,
    FutCo: Future<Output = anyhow::Result<Con>> + Send,
    FutEx: Future<Output = anyhow::Result<()>> + Send,
{
    let results = future::join_all(servers.into_iter().map(|server| {
        let connection_opener = connection_opener.clone();
        let request_executor = request_executor.clone();
        let target = server.clone();
        tokio::spawn(async move {
            let target_id = target.id.clone();
            let connection = connection_opener(target.clone())
                .await
                .with_context(|| format!("error while connecting to {}", target_id))?;
            request_executor(target, connection)
                .await
                .with_context(|| format!("error while executing request on {}", target_id))
        })
    }))
    .await;

    // return the captured errors to the caller, if any
    let results_with_error: Vec<String> = results
        .into_iter()
        .map(|result| result.unwrap_or_else(|err| Err(err.into())))
        .filter_map(Result::err)
        .map(|err| format!("{err:?}"))
        .collect();
    if results_with_error.is_empty() {
        Ok(())
    } else {
        Err(anyhow!("{}", results_with_error.join(", ")))
    }
}

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

use anyhow::Context;

use crate::config::{Configuration, TargetServer};

/// Get the servers that are referenced by the given server ids. These can either be tags or raw server ids.
///
/// # Arguments
/// * `configuration` - The client configuration.
/// * `server_ids` - The input server ids, either being raw ids or tags.
pub(crate) fn select_target_servers<'a>(
    configuration: &'a Configuration,
    server_ids: &Vec<String>,
) -> anyhow::Result<HashSet<&'a TargetServer>> {
    if server_ids.is_empty() {
        // no server ids were given, this indicates that all servers should be used
        return Ok(configuration.servers.iter().collect());
    }

    let mut target_servers = HashSet::<&'a TargetServer>::new();
    for server_id in server_ids {
        match server_id.strip_prefix("t:") {
            Some(requested_tag) => {
                // requested servers by tag (using "t:" prefix which is stripped)
                let tagged_servers =
                    &mut configuration.get_servers_with_tag(&requested_tag.to_string());
                target_servers.extend(tagged_servers.iter());
            }
            None => {
                // requested server by explicit id, try to find it
                let requested_server = configuration
                    .get_server_by_id(server_id)
                    .with_context(|| format!("unable to find server with id {}", server_id))?;
                target_servers.insert(requested_server);
            }
        }
    }

    Ok(target_servers)
}

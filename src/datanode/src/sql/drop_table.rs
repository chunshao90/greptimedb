// Copyright 2022 Greptime Team
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use catalog::DeregisterTableRequest;
use common_error::prelude::BoxedError;
use common_query::Output;
use common_telemetry::info;
use snafu::ResultExt;
use sql::statements::drop::DropTable;
use table::engine::{EngineContext, TableReference};
use table::requests::DropTableRequest;

use crate::error::{self, Result};
use crate::sql::SqlHandler;

impl SqlHandler {
    pub async fn drop_table(&self, req: DropTableRequest) -> Result<Output> {
        let deregister_table_req = DeregisterTableRequest {
            catalog: req.catalog_name.clone(),
            schema: req.schema_name.clone(),
            table_name: req.table_name.clone(),
        };

        let table_reference = TableReference {
            catalog: &req.catalog_name,
            schema: &req.schema_name,
            table: &req.table_name,
        };
        let table_full_name = table_reference.to_string();

        self.catalog_manager
            .deregister_table(deregister_table_req)
            .await
            .map_err(BoxedError::new)
            .context(error::DropTableSnafu {
                table_name: table_full_name.clone(),
            })?;

        let ctx = EngineContext {};
        self.table_engine()
            .drop_table(&ctx, req)
            .await
            .map_err(BoxedError::new)
            .context(error::DropTableSnafu {
                table_name: table_full_name.clone(),
            })?;

        info!("Successfully dropped table: {}", table_full_name);

        Ok(Output::AffectedRows(1))
    }

    pub fn drop_table_to_request(&self, drop_table: DropTable) -> DropTableRequest {
        DropTableRequest {
            catalog_name: drop_table.catalog_name,
            schema_name: drop_table.schema_name,
            table_name: drop_table.table_name,
        }
    }
}

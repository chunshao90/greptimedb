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

use api::v1::*;
use common_error::prelude::StatusCode;
use common_query::Output;
use snafu::prelude::*;

use crate::database::PROTOCOL_VERSION;
use crate::{error, Client, Result};

#[derive(Clone, Debug)]
pub struct Admin {
    name: String,
    client: Client,
}

impl Admin {
    pub fn new(name: impl Into<String>, client: Client) -> Self {
        Self {
            name: name.into(),
            client,
        }
    }

    pub async fn create(&self, expr: CreateExpr) -> Result<AdminResult> {
        let header = ExprHeader {
            version: PROTOCOL_VERSION,
        };
        let expr = AdminExpr {
            header: Some(header),
            expr: Some(admin_expr::Expr::Create(expr)),
        };
        self.do_request(expr).await
    }

    pub async fn do_request(&self, expr: AdminExpr) -> Result<AdminResult> {
        // `remove(0)` is safe because of `do_requests`'s invariants.
        Ok(self.do_requests(vec![expr]).await?.remove(0))
    }

    pub async fn alter(&self, expr: AlterExpr) -> Result<AdminResult> {
        let header = ExprHeader {
            version: PROTOCOL_VERSION,
        };
        let expr = AdminExpr {
            header: Some(header),
            expr: Some(admin_expr::Expr::Alter(expr)),
        };
        self.do_request(expr).await
    }

    pub async fn drop_table(&self, expr: DropTableExpr) -> Result<AdminResult> {
        let header = ExprHeader {
            version: PROTOCOL_VERSION,
        };
        let expr = AdminExpr {
            header: Some(header),
            expr: Some(admin_expr::Expr::DropTable(expr)),
        };

        self.do_request(expr).await
    }

    /// Invariants: the lengths of input vec (`Vec<AdminExpr>`) and output vec (`Vec<AdminResult>`) are equal.
    async fn do_requests(&self, exprs: Vec<AdminExpr>) -> Result<Vec<AdminResult>> {
        let expr_count = exprs.len();
        let req = AdminRequest {
            name: self.name.clone(),
            exprs,
        };

        let resp = self.client.admin(req).await?;

        let results = resp.results;
        ensure!(
            results.len() == expr_count,
            error::MissingResultSnafu {
                name: "admin_results",
                expected: expr_count,
                actual: results.len(),
            }
        );
        Ok(results)
    }

    pub async fn create_database(&self, expr: CreateDatabaseExpr) -> Result<AdminResult> {
        let header = ExprHeader {
            version: PROTOCOL_VERSION,
        };
        let expr = AdminExpr {
            header: Some(header),
            expr: Some(admin_expr::Expr::CreateDatabase(expr)),
        };
        Ok(self.do_requests(vec![expr]).await?.remove(0))
    }
}

pub fn admin_result_to_output(admin_result: AdminResult) -> Result<Output> {
    let header = admin_result.header.context(error::MissingHeaderSnafu)?;
    if !StatusCode::is_success(header.code) {
        return error::DatanodeSnafu {
            code: header.code,
            msg: header.err_msg,
        }
        .fail();
    }

    let result = admin_result.result.context(error::MissingResultSnafu {
        name: "result".to_string(),
        expected: 1_usize,
        actual: 0_usize,
    })?;
    let output = match result {
        admin_result::Result::Mutate(mutate) => {
            if mutate.failure != 0 {
                return error::MutateFailureSnafu {
                    failure: mutate.failure,
                }
                .fail();
            }
            Output::AffectedRows(mutate.success as usize)
        }
    };
    Ok(output)
}

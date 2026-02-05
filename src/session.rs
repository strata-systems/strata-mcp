//! MCP session management.
//!
//! Wraps a stratadb Session with branch/space context, similar to the CLI's SessionState.

use stratadb::{
    BranchDiffResult, Command, ForkInfo, MergeInfo, MergeStrategy, Output, Session, Strata,
};

use crate::error::{McpError, Result};

/// MCP session state.
///
/// Holds both a `Strata` handle (for branch power ops like fork/diff/merge)
/// and a `Session` (for command execution with transactions). Tracks current
/// branch and space context.
pub struct McpSession {
    /// Strata handle for branch power API
    strata: Strata,
    /// Session for command execution
    session: Session,
    /// Current branch context
    branch: String,
    /// Current space context
    space: String,
    /// Whether a transaction is active
    in_transaction: bool,
}

impl McpSession {
    /// Create a new MCP session from a Strata database.
    pub fn new(strata: Strata) -> Self {
        let session = strata.session();
        Self {
            strata,
            session,
            branch: "default".to_string(),
            space: "default".to_string(),
            in_transaction: false,
        }
    }

    /// Get the current branch name.
    pub fn branch(&self) -> &str {
        &self.branch
    }

    /// Get the current space name.
    pub fn space(&self) -> &str {
        &self.space
    }

    /// Whether a transaction is currently active.
    pub fn in_transaction(&self) -> bool {
        self.in_transaction
    }

    /// Switch to a different branch.
    ///
    /// Verifies the branch exists before switching.
    pub fn switch_branch(&mut self, name: &str) -> Result<()> {
        // Check if branch exists
        let exists = match self.session.execute(Command::BranchExists {
            branch: name.into(),
        })? {
            Output::Bool(b) => b,
            _ => {
                return Err(McpError::Internal(
                    "Unexpected output for BranchExists".to_string(),
                ))
            }
        };

        if !exists {
            return Err(McpError::BranchNotFound(name.to_string()));
        }

        self.branch = name.to_string();
        Ok(())
    }

    /// Switch to a different space.
    pub fn switch_space(&mut self, name: &str) {
        self.space = name.to_string();
    }

    /// Execute a command via the session.
    ///
    /// Updates transaction state tracking based on output.
    pub fn execute(&mut self, cmd: Command) -> Result<Output> {
        let output = self.session.execute(cmd)?;

        // Track transaction state changes
        match &output {
            Output::TxnBegun => self.in_transaction = true,
            Output::TxnCommitted { .. } | Output::TxnAborted => self.in_transaction = false,
            _ => {}
        }

        Ok(output)
    }

    /// Fork the current branch to a new branch.
    pub fn fork_branch(&self, destination: &str) -> Result<ForkInfo> {
        self.strata
            .branches()
            .fork(&self.branch, destination)
            .map_err(McpError::from)
    }

    /// Diff two branches.
    pub fn diff_branches(&self, branch_a: &str, branch_b: &str) -> Result<BranchDiffResult> {
        self.strata
            .branches()
            .diff(branch_a, branch_b)
            .map_err(McpError::from)
    }

    /// Merge a source branch into the current branch.
    pub fn merge_branch(&self, source: &str, strategy: MergeStrategy) -> Result<MergeInfo> {
        self.strata
            .branches()
            .merge(source, &self.branch, strategy)
            .map_err(McpError::from)
    }

    /// Get the current branch ID for use in commands.
    pub fn branch_id(&self) -> Option<stratadb::BranchId> {
        Some(self.branch.clone().into())
    }

    /// Get the current space for use in commands.
    pub fn space_id(&self) -> Option<String> {
        Some(self.space.clone())
    }

    /// Get a reference to the underlying Strata database.
    pub fn strata(&self) -> &Strata {
        &self.strata
    }
}

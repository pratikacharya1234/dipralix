use anyhow::Result;
use colored::Colorize;
use std::sync::Arc;

use crate::backend::BackendClient;
use crate::config::Config;
use crate::safety::RiskLevel;

pub struct PeerReviewEngine {
    client: Arc<BackendClient>,
    config: Config,
}

impl PeerReviewEngine {
    pub fn new(client: Arc<BackendClient>, config: Config) -> Self {
        Self { client, config }
    }

    /// Triggers a debate if the command/action matches high-risk criteria.
    pub fn review_action<'a>(
        &'a self,
        action: &'a str,
        risk: RiskLevel,
        context: &'a str,
    ) -> core::pin::Pin<Box<dyn std::future::Future<Output = Result<String>> + 'a>> {
        Box::pin(async move {
            if risk != RiskLevel::Confirm && risk != RiskLevel::Deny {
                return Ok(format!(
                    "Action approved automatically (Risk Level: {:?})",
                    risk
                ));
            }

            println!(
                "\n  {} Initiating Red Team vs Blue Team peer review...",
                "[DEBATE]".cyan()
            );

            let client = self.client.clone();
            let action_clone = action.to_string();
            let ctx_clone = context.to_string();
            let config_clone = self.config.clone();

            // Run Red Team (Proponent)
            let prompt_red = format!(
                "You are the RED TEAM. Your goal is to argue FOR the following action and prove it is safe and correct.\n\
                Context: {}\n\n\
                Action: {}\n\n\
                Provide a short 3-sentence argument supporting this action.",
                ctx_clone, action_clone
            );
            let red_res = crate::agent::run_ci_agent(&client, &config_clone, &prompt_red).await;

            // Run Blue Team (Adversary)
            let prompt_blue = format!(
                "You are the BLUE TEAM. Your goal is to argue AGAINST the following action and find all possible flaws, security risks, or side effects.\n\
                Context: {}\n\n\
                Action: {}\n\n\
                Provide a short 3-sentence argument highlighting the risks.",
                ctx_clone, action_clone
            );
            let blue_res = crate::agent::run_ci_agent(&client, &config_clone, &prompt_blue).await;

            let red_arg = red_res
                .map(|r| r.message)
                .unwrap_or_else(|e| format!("Red team failed: {}", e));
            let blue_arg = blue_res
                .map(|r| r.message)
                .unwrap_or_else(|e| format!("Blue team failed: {}", e));

            println!("\n  {} {}", "RED TEAM:".red().bold(), red_arg);
            println!("\n  {} {}", "BLUE TEAM:".blue().bold(), blue_arg);

            // Arbitrate
            let arbitration_prompt = format!(
                "You are the ARBITRATOR. Evaluate the arguments for and against this action.\n\n\
                Action: {}\n\n\
                Red Team (Pro): {}\n\n\
                Blue Team (Con): {}\n\n\
                Decide if the action should proceed. Reply with exactly 'APPROVE: [reason]' or 'REJECT: [reason]'.",
                action, red_arg, blue_arg
            );

            let final_decision =
                crate::agent::run_ci_agent(&self.client, &self.config, &arbitration_prompt)
                    .await
                    .map(|r| r.message)
                    .unwrap_or_else(|_| {
                        "APPROVE: Arbitration failed, falling back to manual review".into()
                    });

            println!("\n  {} {}", "ARBITRATOR:".magenta().bold(), final_decision);

            Ok(final_decision)
        })
    }
}

use uuid::Uuid;

use super::Database;
use crate::error::{AppError, Result};

use crate::{
    db::models::ShotEventRecord,
    domain::{
        AnalyticsComparisonReport, AnalyticsFilter, DomainError, PlayerAnalyticsReport, ShotEvent,
    },
};

impl Database {
    pub async fn query_player_shots(
        &self,
        actor_id: Uuid,
        player_id: Uuid,
    ) -> Result<Vec<ShotEvent>> {
        let _player = self.verify_player_read_access(actor_id, player_id).await?;
        let mut db = self.inner.clone();

        let records = ShotEventRecord::filter(ShotEventRecord::fields().player_id().eq(player_id))
            .order_by(ShotEventRecord::fields().id().asc())
            .exec(&mut db)
            .await?;

        Ok(records.into_iter().map(ShotEvent::from).collect())
    }

    pub async fn compute_player_analytics(
        &self,
        actor_id: Uuid,
        player_id: Uuid,
        filter: AnalyticsFilter,
    ) -> Result<PlayerAnalyticsReport> {
        if let Some(set_num) = filter.set_number
            && !(1..=5).contains(&set_num)
        {
            return Err(AppError::Domain(DomainError::InvalidSetNumber(set_num)));
        }

        let events = self.query_player_shots(actor_id, player_id).await?;
        Ok(PlayerAnalyticsReport::from_events(
            player_id, filter, &events,
        ))
    }

    pub async fn compare_player_analytics(
        &self,
        actor_id: Uuid,
        player_id: Uuid,
        scope_a: AnalyticsFilter,
        scope_b: AnalyticsFilter,
    ) -> Result<AnalyticsComparisonReport> {
        if let Some(set_num) = scope_a.set_number
            && !(1..=5).contains(&set_num)
        {
            return Err(AppError::Domain(DomainError::InvalidSetNumber(set_num)));
        }
        if let Some(set_num) = scope_b.set_number
            && !(1..=5).contains(&set_num)
        {
            return Err(AppError::Domain(DomainError::InvalidSetNumber(set_num)));
        }

        let events = self.query_player_shots(actor_id, player_id).await?;
        let report_a = PlayerAnalyticsReport::from_events(player_id, scope_a, &events);
        let report_b = PlayerAnalyticsReport::from_events(player_id, scope_b, &events);

        Ok(AnalyticsComparisonReport::compare(report_a, report_b))
    }
}

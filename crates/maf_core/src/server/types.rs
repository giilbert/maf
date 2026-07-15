//! Shared types for MAF Platform API routes.

use maf_schemas::apps::AppNameAndOrgSlug;
use serde::Deserialize;

/// Used to deserialize the path parameters for paths like `/@/{org_slug}/{app_name}/{room_key}`.
#[derive(Debug, Clone, Deserialize)]
pub struct AppRoomPath {
    #[serde(flatten)]
    pub app_id: AppOrgPath,
    pub room_key: String,
}

impl AppRoomPath {
    pub fn app_org(&self) -> AppNameAndOrgSlug {
        self.app_id.app_org()
    }
}

/// Used to deserialize the path parameters for paths like `/@/{org_slug}/{app_name}`.
#[derive(Debug, Clone, Deserialize)]
pub struct AppOrgPath {
    pub org_slug: String,
    pub app_name: String,
}

impl AppOrgPath {
    pub fn app_org(&self) -> AppNameAndOrgSlug {
        AppNameAndOrgSlug {
            app: self.app_name.clone(),
            org: self.org_slug.clone(),
        }
    }
}

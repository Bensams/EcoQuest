export type Role = 'USER' | 'ADMIN';

export type UserProfile = {
  id: string;
  username: string;
  email: string;
  role: Role;
  wallet_address: string | null;
  status: UserStatus;
  eco_points: number;
  created_at: string;
};

/** A summed total for one metric. Mirrors the API's `MetricTotal`. */
export type MetricTotal = { metric: string; unit: string; value: number };

export type ImpactStats = {
  verified_activities: number;
  active_participants: number;
  approved_organizations: number;
  certificates_issued: number;
  metrics: MetricTotal[];
};

export type CommunityGoal = {
  name: string;
  metric: string;
  unit: string;
  target_value: number;
  current_value: number;
};

export type EventImpact = { metric: string; unit: string; expected_value: number };

export type EventStatus =
  | 'DRAFT'
  | 'PUBLISHED'
  | 'ACTIVE'
  | 'COMPLETED'
  | 'CANCELLED'
  | 'SUSPENDED'
  | 'ARCHIVED';

export type ActivityType =
  | 'TREE_PLANTING'
  | 'WASTE_COLLECTION'
  | 'RECYCLING'
  | 'BEACH_CLEANUP'
  | 'ENERGY_SAVING'
  | 'EDUCATION'
  | 'OTHER';

export type EcoEvent = {
  id: string;
  organization_id: string;
  organization_name: string;
  created_by: string | null;
  name: string;
  description: string;
  activity_type: string;
  location: string;
  starts_at: string;
  ends_at: string;
  capacity: number;
  eco_points: number;
  status: EventStatus;
  impacts: EventImpact[];
  registered_count: number;
};

export type ParticipationStatus =
  | 'REGISTERED'
  | 'PENDING_VERIFICATION'
  | 'VERIFIED'
  | 'REJECTED'
  | 'CANCELLED';

export type Participation = {
  id: string;
  event_id: string;
  user_id: string;
  username?: string;
  status: ParticipationStatus;
  registered_at: string;
  checked_in_at: string | null;
};

export type OrganizationStatus = {
  organization_id: string;
  name: string;
  verification_status: VerificationStatus;
  reviewed_at: string | null;
};

export type Organization = {
  id: string;
  name: string;
  organization_type: string;
  location: string;
  description: string;
  verification_status: VerificationStatus;
  owner_id: string;
  reviewed_at: string | null;
  created_at: string;
};

export type Activity = {
  participation: Participation;
  event: EcoEvent;
};

export type AchievementKind = 'PLATFORM' | 'ONCHAIN';

export type Achievement = {
  achievement_key: string;
  title: string;
  description: string;
  kind: AchievementKind;
  /** `EARNED`/`IN_PROGRESS` for platform achievements, the mint status for on-chain ones. */
  status: string;
  progress: number;
  threshold: number;
  earned: boolean;
  verification_reference: string | null;
  wallet_address: string | null;
  mint_identifier: string | null;
  transaction_signature: string | null;
  explorer_url: string | null;
};

export type Certificate = {
  certificate_number: string;
  participation_id: string;
  user_id: string;
  event_id: string;
  event_name: string;
  organization_name: string;
  participant_name: string;
  issued_at: string;
  volunteer_duration_minutes: number;
  verification_hash: string;
  status: string;
};

export type SessionResponse = { user: UserProfile; expires_in: number };

export type VerificationStatus = 'PENDING' | 'APPROVED' | 'REJECTED' | 'SUSPENDED' | 'INACTIVE';
export type UserStatus = 'ACTIVE' | 'SUSPENDED' | 'DELETED';

export type Page<T> = {
  items: T[];
  total: number;
  page: number;
  per_page: number;
};

export type AdminAuditEntry = {
  id: string;
  actor_id: string | null;
  actor_username: string | null;
  action: string;
  entity_type: string;
  entity_id: string | null;
  metadata: Record<string, unknown>;
  created_at: string;
};

export type AdminOrganization = {
  id: string;
  name: string;
  organization_type: string;
  location: string;
  description: string;
  verification_status: VerificationStatus;
  owner_id: string | null;
  owner_username: string | null;
  event_count: number;
  review_reason: string | null;
  supporting_documents: unknown;
  reviewed_at: string | null;
  created_at: string;
};

export type AdminEvent = {
  id: string;
  organization_id: string;
  organization_name: string;
  owner_username: string;
  name: string;
  description: string;
  activity_type: string;
  location: string;
  starts_at: string;
  ends_at: string;
  capacity: number;
  eco_points: number;
  status: EventStatus;
  previous_status: EventStatus | null;
  registered_count: number;
  flagged_count: number;
  cancelled_by: string | null;
  cancelled_at: string | null;
  cancellation_reason: string | null;
  moderation_reason: string | null;
  created_at: string;
};

export type AdminUser = {
  id: string;
  username: string;
  email: string;
  role: Role;
  status: UserStatus;
  eco_points: number;
  wallet_address: string | null;
  status_reason: string | null;
  created_at: string;
};

export type AdminOrgMember = {
  user_id: string;
  username: string;
  email: string;
  member_role: string;
};

export type AdminCertificateSummary = {
  certificate_number: string;
  participation_id: string;
  event_name: string;
  participant_name: string;
  issued_at: string;
  status: string;
};

export type AdminOrganizationDetail = {
  organization: AdminOrganization;
  events: AdminEvent[];
  members: AdminOrgMember[];
  certificates: AdminCertificateSummary[];
  impact: ImpactStats;
  audit: AdminAuditEntry[];
};

export type AdminQrStatus = {
  has_active: boolean;
  activates_at: string | null;
  expires_at: string | null;
  revoked_at: string | null;
};

export type AdminParticipant = {
  id: string;
  event_id: string;
  user_id: string;
  username: string;
  status: ParticipationStatus;
  registered_at: string;
  checked_in_at: string | null;
  flagged: boolean;
  flag_reason: string | null;
  points_awarded: number;
};

export type AdminEventDetail = {
  event: AdminEvent;
  impacts: EventImpact[];
  participants: AdminParticipant[];
  qr: AdminQrStatus;
  impact: ImpactStats;
  audit: AdminAuditEntry[];
};

export type AdminPointTransaction = {
  id: string;
  amount: number;
  reason: string;
  participation_id: string | null;
  created_by: string | null;
  created_at: string;
};

export type AdminUserActivity = {
  participation_id: string;
  event_id: string;
  event_name: string;
  status: ParticipationStatus;
  registered_at: string;
  checked_in_at: string | null;
  points_awarded: number;
};

export type AdminUserAchievement = {
  achievement_key: string;
  status: string;
  verification_reference: string;
};

export type AdminUserDetail = {
  user: AdminUser;
  activities: AdminUserActivity[];
  certificates: AdminCertificateSummary[];
  achievements: AdminUserAchievement[];
  ledger: AdminPointTransaction[];
  audit: AdminAuditEntry[];
};

export type PasswordResetLink = {
  user_id: string;
  reset_url: string;
  expires_at: string;
};

export type PointCorrection = {
  user_id: string;
  amount: number;
  eco_points: number;
};

export type EventModeration = 'CANCEL' | 'SUSPEND' | 'RESTORE' | 'ARCHIVE';

export type VerificationResult = {
  participation_id: string;
  status: ParticipationStatus;
  points_awarded: number;
  player_eco_points: number;
  player_level: number;
};

export type VerificationBatchResponse = { results: VerificationResult[] };

export type QrResponse = {
  code: string;
  activates_at: string;
  expires_at: string;
  svg: string;
};
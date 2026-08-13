export type Role = 'USER' | 'ADMIN';

export type UserProfile = {
  id: string;
  username: string;
  email: string;
  role: Role;
  wallet_address: string | null;
  status: 'ACTIVE' | 'SUSPENDED';
  eco_points: number;
  created_at: string;
};

export type ImpactMetric = { metric: string; unit: string; value: number };

export type ImpactStats = {
  verified_activities: number;
  active_participants: number;
  approved_organizations: number;
  certificates_issued: number;
  metrics: ImpactMetric[];
};

export type CommunityGoal = {
  name: string;
  metric: string;
  unit: string;
  target_value: number;
  current_value: number;
};

export type EventImpact = { metric: string; unit: string; expected_value: number };

export type EventStatus = 'DRAFT' | 'PUBLISHED' | 'ACTIVE' | 'COMPLETED' | 'CANCELLED';

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
  verification_status: 'PENDING' | 'APPROVED' | 'REJECTED' | 'SUSPENDED';
  reviewed_at: string | null;
};

export type Organization = {
  id: string;
  name: string;
  organization_type: string;
  location: string;
  description: string;
  verification_status: 'PENDING' | 'APPROVED' | 'REJECTED' | 'SUSPENDED';
  owner_id: string;
  reviewed_at: string | null;
  created_at: string;
};

export type Activity = {
  participation: Participation;
  event: EcoEvent;
};

export type Achievement = {
  achievement_key: string;
  status: string;
  verification_reference: string;
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

export type VerificationStatus = 'PENDING' | 'APPROVED' | 'REJECTED' | 'SUSPENDED';
export type UserStatus = 'ACTIVE' | 'SUSPENDED' | 'DELETED';

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
  reviewed_at: string | null;
  created_at: string;
};

export type AdminEvent = {
  id: string;
  organization_id: string;
  organization_name: string;
  owner_username: string;
  name: string;
  activity_type: string;
  location: string;
  starts_at: string;
  ends_at: string;
  status: EventStatus;
  registered_count: number;
  cancelled_by: string | null;
  cancelled_at: string | null;
  cancellation_reason: string | null;
  created_at: string;
};

export type AdminUser = {
  id: string;
  username: string;
  email: string;
  role: Role;
  status: UserStatus;
  eco_points: number;
  created_at: string;
};

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
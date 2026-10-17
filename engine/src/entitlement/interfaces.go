package entitlement

// Tier represents the license subscription level
type Tier string

const (
	TierCommunity       Tier = "community"
	TierTeam            Tier = "team"
	TierDockyard        Tier = "dockyard"
	TierPrivateDockyard Tier = "private_dockyard"
)

// QuotaPolicy defines the resource limits and delegation depth ceiling
type QuotaPolicy interface {
	Tier() Tier
	MaxFleets() int
	MaxShips() int
	MaxSquadsTotal() int
	MaxCrewsPerSquad() int
	MaxCrewsPerShip() int
	MaxConcurrentVoyages() int
	MaxDelegationDepth() *int
}

// UsageReport reports current resource counts versus entitlement ceilings
type UsageReport struct {
	Tier                 Tier `json:"tier"`
	FleetsCount          int  `json:"fleets_count"`
	MaxFleets            int  `json:"max_fleets"`
	ShipsCount           int  `json:"ships_count"`
	MaxShips             int  `json:"max_ships"`
	SquadsCount          int  `json:"squads_count"`
	MaxSquadsTotal       int  `json:"max_squads_total"`
	CrewsCount           int  `json:"crews_count"`
	MaxCrewsPerShip      int  `json:"max_crews_per_ship"`
	ConcurrentVoyages    int  `json:"concurrent_voyages"`
	MaxConcurrentVoyages int  `json:"max_concurrent_voyages"`
	DelegationDepthCap   *int `json:"delegation_depth_cap"`
}

package entitlement

func intPtr(val int) *int {
	return &val
}

// CommunityPolicy represents the default local tier
// Limits: {MaxFleets: 1, MaxShips: 1, MaxSquadsTotal: 2, MaxCrewsPerSquad: 6, MaxCrewsPerShip: 8, MaxConcurrentVoyages: 2, MaxDelegationDepth: 1}
type CommunityPolicy struct{}

func NewCommunityPolicy() QuotaPolicy        { return &CommunityPolicy{} }
func (p *CommunityPolicy) Tier() Tier        { return TierCommunity }
func (p *CommunityPolicy) MaxFleets() int    { return 1 }
func (p *CommunityPolicy) MaxShips() int     { return 1 }
func (p *CommunityPolicy) MaxSquadsTotal() int { return 2 }
func (p *CommunityPolicy) MaxCrewsPerSquad() int { return 6 }
func (p *CommunityPolicy) MaxCrewsPerShip() int  { return 8 }
func (p *CommunityPolicy) MaxConcurrentVoyages() int { return 2 }
func (p *CommunityPolicy) MaxDelegationDepth() *int { return intPtr(1) }

// TeamPolicy represents the multi-ship workgroup tier
// Limits: {MaxFleets: 1, MaxShips: 3, MaxSquadsTotal: 6, MaxCrewsPerSquad: 6, MaxCrewsPerShip: 22, MaxConcurrentVoyages: 4, MaxDelegationDepth: 2}
type TeamPolicy struct{}

func NewTeamPolicy() QuotaPolicy             { return &TeamPolicy{} }
func (p *TeamPolicy) Tier() Tier             { return TierTeam }
func (p *TeamPolicy) MaxFleets() int         { return 1 }
func (p *TeamPolicy) MaxShips() int          { return 3 }
func (p *TeamPolicy) MaxSquadsTotal() int    { return 6 }
func (p *TeamPolicy) MaxCrewsPerSquad() int  { return 6 }
func (p *TeamPolicy) MaxCrewsPerShip() int   { return 22 }
func (p *TeamPolicy) MaxConcurrentVoyages() int { return 4 }
func (p *TeamPolicy) MaxDelegationDepth() *int { return intPtr(2) }

// DockyardPolicy represents the high-scale fleet organization tier
// Limits: {MaxFleets: 1, MaxShips: 6, MaxSquadsTotal: 12, MaxCrewsPerSquad: 8, MaxCrewsPerShip: 100, MaxConcurrentVoyages: 8, MaxDelegationDepth: 3}
type DockyardPolicy struct{}

func NewDockyardPolicy() QuotaPolicy         { return &DockyardPolicy{} }
func (p *DockyardPolicy) Tier() Tier         { return TierDockyard }
func (p *DockyardPolicy) MaxFleets() int     { return 1 }
func (p *DockyardPolicy) MaxShips() int      { return 6 }
func (p *DockyardPolicy) MaxSquadsTotal() int { return 12 }
func (p *DockyardPolicy) MaxCrewsPerSquad() int { return 8 }
func (p *DockyardPolicy) MaxCrewsPerShip() int  { return 100 }
func (p *DockyardPolicy) MaxConcurrentVoyages() int { return 8 }
func (p *DockyardPolicy) MaxDelegationDepth() *int { return intPtr(3) }

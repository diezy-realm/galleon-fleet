package identity

import "github.com/google/wire"

// Set defines Wire provider set for the identity domain
var Set = wire.NewSet(
	NewMemoryTokenStore,
	NewDenyByDefaultResolver,
	wire.Bind(new(PrincipalResolver), new(*DenyByDefaultResolver)),
	NewPairingService,
	NewPairingHTTPHandler,
)

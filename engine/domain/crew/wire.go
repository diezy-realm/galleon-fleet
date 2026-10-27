package crew

import "github.com/google/wire"

// Set defines Wire dependency injection provider set for domain/crew
var Set = wire.NewSet(
	NewStandardTokenCounter,
	NewContextResolver,
	NewTurnEngine,
	wire.Bind(new(TokenCounter), new(*StandardTokenCounter)),
	wire.Bind(new(TurnLifecycle), new(*TurnEngine)),
)

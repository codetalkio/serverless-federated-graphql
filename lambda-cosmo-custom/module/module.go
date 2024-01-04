package module

import (
	"net/http"
	"net/url"
	"os"
	"strings"

	"github.com/wundergraph/cosmo/router/core"
)

func init() {
	// Register your module here
	core.RegisterModule(&OverrideSubgraphUrlModule{})
}

const moduleId = "overrideSubgraphUrl"

type OverrideSubgraphUrlModule struct{}

func (m *OverrideSubgraphUrlModule) OnOriginRequest(request *http.Request, ctx core.RequestContext) (*http.Request, *http.Response) {
	subgraph := ctx.ActiveSubgraph(request)

	// Lookup if the subgraph URL has been overridden by an environment variable in the shape of `SUBGRAPH_<SUBGRAPH_NAME>_URL`.
	subgraphEnvName := "SUBGRAPH_" + strings.ToUpper(subgraph.Name) + "_URL"
	subgraphEnv, subgraphEnvIsSet := os.LookupEnv(subgraphEnvName)
	if subgraphEnvIsSet {
		subgraphUrl, err := url.Parse(subgraphEnv)
		ctx.Logger().Info("[OverrideSubgraphUrl] Setting URL to '" + subgraphEnv + "' for subgraph '" + strings.ToUpper(subgraph.Name) + "'")
		if err != nil {
			ctx.Logger().Error("[OverrideSubgraphUrl] Failed to parse URL '" + subgraphEnv + "' from environment variable '" + subgraphEnvName + "'")
		}
		// Override both the request URL/Host as well as the Subgraph URL.
		request.URL = subgraphUrl
		request.Host = subgraphUrl.Host
		subgraph.Url = subgraphUrl
	}

	return request, nil
}

func (m *OverrideSubgraphUrlModule) Module() core.ModuleInfo {
	return core.ModuleInfo{
		// This is the ID of your module, it must be unique
		ID: moduleId,
		New: func() core.Module {
			return &OverrideSubgraphUrlModule{}
		},
	}
}

// Interface guard
var (
	_ core.EnginePreOriginHandler = (*OverrideSubgraphUrlModule)(nil)
)

local json = require("json")

local _M = {}

function _M.radarr_scan(path, api_addr, api_token)
	if not path then
		_trahl.log(_trahl.ERROR, "Radarr scan failed: missing path")
		return false
	end

	local api_addr  = api_addr  or _trahl.vars.RADARR_ADDR
	local api_token = api_token or _trahl.vars.RADARR_API_TOKEN

	if not api_token or not api_addr then
		_trahl.log(_trahl.ERROR, "Radarr scan failed: RADARR_API_TOKEN or RADARR_ADDR not set")
		return false
	end

	local api_addr = api_addr:gsub("/$", "")
	local url = api_addr .. "/api/v3/command"

	local headers = {
		["Content-Type"] = "application/json",
		["X-Api-Key"] = api_token
	}

	local body = json.encode({
		name = "DownloadedMoviesScan",
		path = path
	})

	local ok, status, resp = pcall(function()
		return _trahl.http_request("POST", url, headers, body)
	end)

	if not ok then
		_trahl.log(_trahl.ERROR, "http_request pcall error")
		return false
	end

	local success = status >= 200 and status < 300
	if not success then
		_trahl.log(_trahl.ERROR, "Radarr scan failed: status " .. tostring(status) .. " body: " .. tostring(resp))
	else
		_trahl.log(_trahl.INFO, "Radarr scan triggered successfully for path: " .. path)
	end

	return success
end

function _M.sonarr_scan(path, api_addr, api_token)
	if not path then
		_trahl.log(_trahl.ERROR, "Sonarr scan failed: missing path")
		return false
	end

	local api_addr  = api_addr  or _trahl.vars.SONARR_ADDR
	local api_token = api_token or _trahl.vars.SONARR_API_TOKEN

	if not api_token or not api_addr then
		_trahl.log(_trahl.ERROR, "Sonarr scan failed: SONARR_API_TOKEN or SONARR_ADDR not set")
		return false
	end

	local api_addr = api_addr:gsub("/$", "")
	local url = api_addr .. "/api/v3/command"

	local headers = {
		["Content-Type"] = "application/json",
		["X-Api-Key"] = api_token
	}

	local body = json.encode({
		name = "DownloadedMoviesScan",
		path = path
	})

	local ok, status, resp = pcall(function()
		return _trahl.http_request("POST", url, headers, body)
	end)

	if not ok then
		_trahl.log(_trahl.ERROR, "http_request pcall error")
		return false
	end

	local success = status >= 200 and status < 300
	if not success then
		_trahl.log(_trahl.ERROR, "Sonarr scan failed: status " .. tostring(status) .. " body: " .. tostring(resp))
	else
		_trahl.log(_trahl.INFO, "Sonarr scan triggered successfully for path: " .. path)
	end

	return success
end

function _M.discord_message(str, webhook)
	local url = webhook or _trahl.vars.DISCORD_WEBHOOK

	if not url or not str then
		_trahl.log(_trahl.ERROR, "Discord webhook failed: webhook or message not set")
		return false
	end

    local headers = {
        ["Content-Type"] = "application/json"
    }

    local raw_body = json.encode({ content = str })

    local ok, status, body = pcall(function()
        return _trahl.http_request("POST", url, headers, raw_body)
    end)

	if not ok then
		_trahl.log(_trahl.ERROR, "Discord webhook failed (pcall error)")
		return false
	end

	local success = status >= 200 and status < 300
	if not success then
		_trahl.log(_trahl.ERROR, "Failed to send message to discord: status: " .. tostring(status) .. " body: " .. tostring(body))
	end

    return success
end

return _M

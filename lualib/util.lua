local json = require("json")

local _M = {}

function _M.file_size(path)
	local f = io.open(path, "rb")
	if not f then return nil end
	local size = f:seek("end")
	f:close()
	return size
end

function _M.print_table(t)
    for k, v in pairs(t) do
        if type(v) == "table" then
            print(k .. ":")
            _M.print_table(v)
        else
            print(k, v)
        end
    end
end

function _M.discord_message(url, str)
	if not url or not str then
		_trahl.log(_trahl.ERROR, "Discord webhook failed: invalid url or message")
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

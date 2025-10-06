local json = require("json")

local _M = {}

function _M.file_size(path)
	local f = io.open(path, "rb")
	if not f then return nil end
	local size = f:seek("end")
	f:close()
	return size
end

function _M.file_name(path)
	return path:match("^.+/(.+)$")
end

function _M.strip_ext(filename)
	return filename:match("(.+)%..+$") or filename
end

function _M.random_string(len)
	local charset = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    local s = {}
	math.randomseed(os.time())
    for i = 1, len do
        local idx = math.random(1, #charset)
        s[i] = charset:sub(idx, idx)
    end
    return table.concat(s)
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

function _M.panic(str)
	_trahl.log(_trahl.ERROR, str)
	error(str)
end

function _M.matches_regex(text, pattern)
    if not pattern or pattern == "" then
        return false
    end
	
	local ok, matched = pcall(function()
        return _trahl.regex_match(text, pattern)
    end)
    return ok and matched
end

return _M

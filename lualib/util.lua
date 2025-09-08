local _M = {}

function _M.file_size(path)
	local f = io.open(path, "rb")
	if not f then return nil end
	local size = f:seek("end")
	f:close()
	return size
end

function print_table(t)
    for k, v in pairs(t) do
        if type(v) == "table" then
            print(k .. ":")
            print_table(v)
        else
            print(k, v)
        end
    end
end

return _M

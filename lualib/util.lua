local _M = {}

function _M.file_size(path)
	local f = io.open(path, "rb")
	if not f then return nil end
	local size = f:seek("end")
	f:close()
	return size
end

return _M

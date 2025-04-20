local M = {}

local function execute_command(command)
    local handle = io.popen(command)
    local result = handle:read("*a")
    handle:close()
    return result
end

function M.get_last_commit_message(path, limit)
    local command = string.format("git log -1 --pretty=format:%%s -- %s", path)
    local message = execute_command(command):gsub("\n", "")
    if #message > limit then
        return message:sub(1, limit) .. ".."
    end
end

function M.get_main_language(path)
    local command = string.format("github-linguist --json %s", path)
    local result = execute_command(command)
    local data = result:match('"language":%s*"([^"]+)"')
    return data or "Unknown"
end

function M.get_last_commit_time(path)
    local command = string.format("git log -1 --format=%%cd --date=relative -- %s", path)
    return execute_command(command):gsub("\n", "")
end

function M.get_last_committer(path)
    local command = string.format("git log -1 --format=%%an -- %s", path)
    return execute_command(command):gsub("\n", "")
end

return M

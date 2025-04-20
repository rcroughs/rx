local utils = require("utils")

local M = {}

function M.init(limit)
    M.limit = limit
end

function M.get_last_commit_message(entry)
    return utils.get_last_commit_message(entry.path, M.limit)
end

function M.get_main_language(entry)
    return utils.get_main_language(entry.path)
end

function M.get_last_commit_time(entry)
    return utils.get_last_commit_time(entry.path)
end

function M.get_last_committer(entry)
    return utils.get_last_committer(entry.path)
end

return M

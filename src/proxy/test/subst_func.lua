local function add_version(name)
    return name .. '/0.0.0-example'
end

target("example.com:3000")
    :req(sub(header("user-agent"), add_version))
    :resp(sub(header("server"), add_version))

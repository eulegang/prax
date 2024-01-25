target("example.com:3000")
    :req(set(header("Authentication"), "Bearer foobarxyz"))
    :resp(set(header("server"), "foobar"))

print("hello from config!")

focus()

target('localhost:8000')
    :req(
      set_header("Authorization", "Bearer xyz"),
      log(method),
      log(path)
    )
    :resp(
      set_header("X-AttackProxy", "set"),
      log(method),
      log(status),
      log(body)
    )

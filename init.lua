focus()

target('localhost:8000')
    :req(
      set_header("Authorization", "Bearer xyz")
    )
    :resp(
      set_header("X-AttackProxy", "set")
    )

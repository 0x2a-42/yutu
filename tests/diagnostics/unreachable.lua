function If_statement()
  if false then
    print("unreachable in if")
  end
end

function While_loop()
  while true do
    -- loop forever
  end
  print("unreachable after while")
end

function Repeat_loop()
  repeat
    -- loop forever
  until false
  print("unreachable after repeat")
end

function After_return()
  if true then
    return
  end
  print("unreachable after return")
end

function After_break()
  while true do
    break
    print("unreachable after break")
  end
end

function After_assert()
  assert(false)
  print("unreachable after assert")
end

function After_error()
  error("")
  print("unreachable after error")
end

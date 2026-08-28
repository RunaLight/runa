local runa = require("runa")
local Transform = runa.Transform
local SpriteRenderer = runa.SpriteRenderer

function start(ctx: runa.ScriptContext)
    print("[player_move] start fired -> emit player_started")
    ctx.events[#ctx.events + 1] = { name = "player_started", x = 0, y = 0 }
end

function update(ctx: runa.ScriptContext)
    local t = ctx:GetComponent(Transform)
    if t then
        local dir = runa.vec2(0, 0)
        if ctx.input:is_key_pressed("KeyA") then dir.x = dir.x - 1 end
        if ctx.input:is_key_pressed("KeyD") then dir.x = dir.x + 1 end
        if ctx.input:is_key_pressed("KeyW") then dir.y = dir.y + 1 end
        if ctx.input:is_key_pressed("KeyS") then dir.y = dir.y - 1 end


        if runa.length2(dir) > 0 then
            dir = runa.normalize2(dir)
        end

        local speed = 20.0
        t.position.x = t.position.x + dir.x * speed * ctx.dt
        t.position.y = t.position.y + dir.y * speed * ctx.dt

        -- Emit a movement event every frame the player is moving (observe it from a
        -- Rust `EventBus` subscriber, or just watch the position print below).
        ctx.events[#ctx.events + 1] = { name = "player_moved", x = t.position.x, y = t.position.y }
        print(string.format("[player_move] moved -> x=%.2f y=%.2f", t.position.x, t.position.y))
    end

    -- Edge-triggered feedback: fire + print only on the frame the key goes down.
    if ctx.input:is_key_just_pressed("KeyA") then
        print("[player_move] A just pressed -> emit flip event")
        ctx.events[#ctx.events + 1] = { name = "flip", x = 0, y = 0 }
    end

    local sprite = ctx:GetComponent(SpriteRenderer)
    if sprite then
        sprite.flip_x = ctx.input:is_key_pressed("KeyA")
    end
end

return {
    start = start,
    update = update,
}

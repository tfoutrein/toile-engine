-- Enemy patrol behavior
-- Uses globals set by the engine: patrol_left, patrol_right, pos_x, pos_y
-- Writes back: vel_x

local speed = 60

function on_create(entity)
    engine.log("Enemy " .. entity .. " spawned, patrolling")
end

function on_update(entity, dt)
    local x = pos_x or 0
    local left = patrol_left or 0
    local right = patrol_right or 400

    if x >= right then
        speed = -math.abs(speed)
    elseif x <= left then
        speed = math.abs(speed)
    end

    vel_x = speed
end

function on_destroy(entity)
    engine.log("Enemy " .. entity .. " destroyed")
end

return { on_create = on_create, on_update = on_update, on_destroy = on_destroy }

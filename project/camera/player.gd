extends CharacterBody3D

# Speed variables, adjust as needed
@export var speed = 0.5
var sensitivity = 2
@onready var rotator = $rotator
@onready var camera_3d = $rotator/Camera3D

# Terrain system references
var chunk_controller = null
var last_chunk_x = 0
var last_chunk_z = 0
var chunk_size = 32.0  # Make sure this matches the value in your Rust code // TODO, make a central point where this data is set and sendt to the correct components

signal player_chunk_changed(chunk_x: int, chunk_z: int)

func _ready():
	var terrain_system = get_node_or_null("/root/TerrainSystem")
	
	if terrain_system:
		chunk_controller = terrain_system.get_node_or_null("ChunkController")
	else:
		chunk_controller = get_node_or_null("/root/ChunkController")
	
	if chunk_controller:
		var success = chunk_controller.connect_player_signal(self)
		if success:
			# Store initial chunk coordinates
			last_chunk_x = floor(global_position.x / chunk_size)
			last_chunk_z = floor(global_position.z / chunk_size)
			print("Connected to terrain system")
		else:
			push_error("Failed to connect player signal to ChunkController")
	else:
		push_error("Could not find ChunkController node")
		

func _physics_process(_delta):
	var local_velocity = Vector3()
	
	if Input.is_action_pressed("move_forward"):
		local_velocity -= rotator.global_transform.basis[2] * speed
	if Input.is_action_pressed("move_backward"):
		local_velocity += rotator.global_transform.basis[2] * speed
	if Input.is_action_pressed("move_left"):
		local_velocity -= rotator.global_transform.basis[0] * speed
	if Input.is_action_pressed("move_right"):
		local_velocity += rotator.global_transform.basis[0] * speed
	
	var velocity_up = Vector3()
	if Input.is_action_pressed("move_up"):
		velocity_up += self.transform.basis[1] * speed
	if Input.is_action_pressed("move_down"):
		velocity_up -= self.transform.basis[1] * speed
	
	self.translate(local_velocity + velocity_up)
	
	# Check if we've moved to a new chunk
	update_terrain_if_needed()

func _unhandled_input(event):
	if event is InputEventMouseButton: 
		Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	elif event.is_action_pressed("ui_cancel"): 
		Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
	if Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED: 
		if event is InputEventMouseMotion: 
			rotator.rotate_y(-event.relative.x * 0.01)
			camera_3d.rotate_x(-event.relative.y * 0.01)

# Efficiently update the terrain system only when crossing chunk boundaries
func update_terrain_if_needed():
	if chunk_controller == null:
		return
		
	var current_chunk_x = floor(global_position.x / chunk_size)
	var current_chunk_z = floor(global_position.z / chunk_size)
	
	if current_chunk_x != last_chunk_x or current_chunk_z != last_chunk_z:
		last_chunk_x = current_chunk_x
		last_chunk_z = current_chunk_z
		
		emit_signal("player_chunk_changed", current_chunk_x, current_chunk_z)
		print("Signal emitted: player moved to chunk: ", current_chunk_x, ", ", current_chunk_z)

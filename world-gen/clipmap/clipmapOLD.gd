extends Node3D

@export var player : Node3D
@onready var ground_mesh = $GroundMesh

var timer = Timer.new()
var player_pos : Vector3
var snap_step = 20

# Called when the node enters the scene tree for the first time.
func _ready():
	add_child(timer)
	timer.connect("timeout", Callable(self, "snap"))
	timer.set_wait_time(1)
	snap()

func snap():
	player_pos = player.global_transform.origin.snapped(Vector3(snap_step, 0, snap_step))
	global_position = player.global_transform.origin.round() * Vector3(1, 0, 1)
	update_shader_params()
	timer.start()

func update_shader_params():
	#var div = 2000.0
	#var material = ground_mesh.get_surface_override_material(0)
	#material.set_shader_param("uv_offset", Vector2(player_pos.x / div, player_pos.z / div))

# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	#var new_player_pos = player.global_transform.origin.snapped(Vector3(snap_step, 0, snap_step))
	#if new_player_pos != player_pos:
		#player_pos = new_player_pos
		#update_shader_params()

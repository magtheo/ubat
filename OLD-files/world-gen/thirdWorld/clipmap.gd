extends Node3D

@onready var ground_mesh = $GroundMesh
@onready var player = $"../../../CameraController"

var timer = Timer.new()
var player_pos : Vector3
var snap_step = 20

# Called when the node enters the scene tree for the first time.
func _ready():
	add_child(timer)
	timer.connect("timeout", Callable(self,"snap"))
	timer.set_wait_time(1)
	snap()


func snap():
	$Lable.text = str(Engine.get_frames_per_second())
	var div = 2000
	player_pos = player.global_transform.origin.snapped(Vector3(snap_step, 0, snap_step))
	ground_mesh.global_transform.origin.x = player_pos.x
	ground_mesh.global_transform.origin.z = player_pos.z
	ground_mesh.get_surface_override_material(0).set("shader_param/uvx", player_pos.x/div)
	ground_mesh.get_surface_override_material(0).set("shader_param/uvy", player_pos.z/div)
	#ground_mesh.get_surface_override_material(0).set("shader_param/uv", player_pos/div)
	#ground_mesh.get_surface_override_material(0).set()
	timer.start()
	
	
	
# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass

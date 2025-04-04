extends CharacterBody3D

# Speed variables, adjust as needed
@export var speed = 2
var sensitivity = 2

@onready var rotator = $rotator
@onready var camera_3d = $rotator/Camera3D

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
	# print("player position", self.position)

func _unhandled_input(event):
	if event is InputEventMouseButton: 
		Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)
	elif event.is_action_pressed("ui_cancel"): 
		Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
	if Input.get_mouse_mode()==Input.MOUSE_MODE_CAPTURED: 
		if event is InputEventMouseMotion: 
			rotator.rotate_y(-event.relative.x * 0.01)
			camera_3d.rotate_x(-event.relative.y * 0.01)

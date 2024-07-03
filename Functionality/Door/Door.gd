extends Node3D

@onready var door = $"."
@onready var handle = $Door/Torus_001
@onready var interactObj = $Door/InteractionV2
@onready var marker =$Door/Torus_001
@onready var view = $"Door/2DView"
@onready var rotateHandle = 0
@onready var curr_mouse_pos_z = 0
@onready var curr_mouse_pos_x = 0 
@onready var angle_mouse = 0
@onready var prev_angle_mouse = 0
@onready var delta_angle_mouse = 0
@onready var viewport = get_viewport() 
@onready var mouse_rotator = $MouseRotation
@onready var rot_object = $Door/Torus_001
# Called when the node enters the scene tree for the first time.
func _ready():
	mouse_rotator.rotation_object = rot_object
	mouse_rotator.axis = "y"
	mouse_rotator.multiplier = 1
	interactObj.assign_and_connect("hei", [Callable(self, "_wheel_rotate")], ["interact"])
	#interactObj.assign_and_connect("hei", Callable(self,"_door_interact"))


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass

func _wheel_rotate(): 
	while (true):
		
		mouse_rotator.rotate_object_to_mouse()
		#mouse_rotator.rotation_object.rotation.y = clamp(mouse_rotator.rotation_object.rotation.y, 0, deg_to_rad(720))
		if (Input.is_action_just_pressed("interact") and 1==0): 
			break; 
		

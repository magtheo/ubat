extends Node3D

@onready var interact_obj = $InteractionV2
@onready var lever_speed = $basic_Controls/Cylinder_002
@onready var lever_descent = $basic_Controls/Cylinder_003
@onready var steering_wheel = $basic_Controls/Node3D
@onready var speed_inc = 0.2
@onready var speed_wheel_rotate = 0.2
# Called when the node enters the scene tree for the first time.
func _ready():
	interact_obj.assign_and_connect(["Press E to control ship", "Q and R to rotate, Y and H to control ascent, mousewheel to accelerate"], [Callable(self, "_submarine_controls")], ["interact"])
	

# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass
func _submarine_controls():	
	if (Input.is_action_pressed("accelerate")): 
		lever_speed.rotate_z(deg_to_rad(speed_inc))
	if (Input.is_action_pressed("decelerate")):
		lever_speed.rotate_z(deg_to_rad(-speed_inc))
	
	if (Input.is_action_pressed("rotate_right")):
		steering_wheel.rotate_object_local(Vector3(0,1,0), deg_to_rad(speed_wheel_rotate))
	if (Input.is_action_pressed("rotate_left")):
		steering_wheel.rotate_object_local(Vector3(0,1,0), deg_to_rad(-speed_wheel_rotate))
	
		
		
	
	

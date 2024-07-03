extends Node3D
@onready var curr_mouse_pos_z = 0
@onready var curr_mouse_pos_x = 0 
@onready var angle_mouse = 0
@onready var prev_angle_mouse = 0
@onready var delta_angle_mouse = 0
@onready var viewport = get_viewport() 
@onready var rotation_object = null
@onready var axis = ""
@onready var multiplier = 1
@onready var angle_change =0
# Called when the node enters the scene tree for the first time.


# Called every frame. 'delta' is the elapsed time since the previous frame.
func rotate_object_to_mouse():
	curr_mouse_pos_x = viewport.get_mouse_position().x-(viewport.get_size().x)/2
	curr_mouse_pos_z = viewport.get_mouse_position().y-(viewport.get_size().y)/2
	if(curr_mouse_pos_x!=0):angle_mouse = atan((curr_mouse_pos_z)/(curr_mouse_pos_x)) 
	
	delta_angle_mouse = angle_mouse-prev_angle_mouse
	prev_angle_mouse = angle_mouse
	angle_change = -clamp((delta_angle_mouse),deg_to_rad(-1*multiplier),deg_to_rad(1*multiplier))
	if (axis == "y"):
		rotation_object.rotate_y(angle_change)
	elif (axis == "x"): 
		rotation_object.rotate_x(angle_change)
	elif (axis == "z"): 
		rotation_object.rotate_z(angle_change)

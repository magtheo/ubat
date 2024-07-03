extends MeshInstance3D


@onready var interactObj = $InteractionV2
@onready var can_interact = false
@onready var box = $"."
# Called when the node enters the scene tree for the first time.
func _ready():
	print("hei")
	
	interactObj.assign_and_connect("hei", [Callable(self,"box_interact"), Callable(self,"box_interact2")], ["interact","interact2"])
	


# Called every frame. 'delta' is the elapsed time since the previous frame.
func _process(delta):
	pass
func box_interact():
	box.rotation.y+=deg_to_rad(30)

func box_interact2(): 
	box.rotation.x+=deg_to_rad(30) 



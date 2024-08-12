extends Node3D

var branches = [] # List of branches
var num_branches = 10
var length = 50

# Called when the node enters the scene tree for the first time.
func _ready():
	for i in range(num_branches):
		var branch = Node3D.new() # Create new branch node
		branches.append(branch)
		add_child(branch) # Add to current scene
		generate_branch(i, length / num_branches, branch) # Generate branch

func generate_branch(n, size, parent):
	var angle = (360.0 / num_branches) * n
	var rot = Basis() # Identity rotation
	rot = rot.rotated(Vector3.UP, deg_to_rad(angle)) # Rotate around the Y-axis
	
	var position = Vector3(length * cos(deg_to_rad(angle)), 0, length * sin(deg_to_rad(angle)))
	parent.global_transform = Transform3D(rot, position)
	
	var branch_mesh = MeshInstance3D.new()
	var mesh = CapsuleMesh.new()
	branch_mesh.mesh = mesh
	branch_mesh.scale = Vector3(1, size, 1) # Make the branch a bit taller than wide
	parent.add_child(branch_mesh)

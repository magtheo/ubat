extends Node3D

signal noises_randomized

# Adjust to your actual resource paths:
const PATH_CORRAL   = "res://Noise/CorralNoise.tres"
const PATH_SAND     = "res://Noise/SandNoise.tres"
const PATH_ROCK     = "res://Noise/RockNoise.tres"
const PATH_KELP     = "res://Noise/KelpNoise.tres"
const PATH_LAVAROCK = "res://Noise/LavaRockNoise.tres"

const PATH_SECTION  = "res://Noise/SectionNoise.tres"
const PATH_BLEND    = "res://Noise/BlendNoise.tres"

var corral_noise
var sand_noise
var rock_noise
var kelp_noise
var lavarock_noise
var section_noise
var blend_noise

@export var seed = 12345

func _ready():
	corral_noise = load(PATH_CORRAL)
	sand_noise = load(PATH_SAND)
	rock_noise = load(PATH_ROCK)
	kelp_noise = load(PATH_KELP)
	lavarock_noise = load(PATH_LAVAROCK)
	section_noise = load(PATH_SECTION)
	blend_noise = load(PATH_BLEND)

func randomize_noises():
	seed = randi_range(0, 100000)
	randomize()
	emit_signal("noises_randomized")

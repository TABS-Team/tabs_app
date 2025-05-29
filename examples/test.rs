// Kolla på wayland window merging
use std::any::Any;

use bevy::prelude::*;

fn main() {
    //Foo::spawn(bar, bundle);
    let mut app = App::new();

    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window::default()),
        ..default()
    }))
    .add_systems(Startup, setup)
    .run();
}

pub fn setup(mut cmds: Commands) {
    spawner_test(&mut cmds, |parent| {
        spawner_test(parent, |row| {});
    });
}

pub fn spawner_test<'a, F>(spawner: &'a mut impl Spawner<'a>, children: F)
where
    F: FnOnce(&mut ChildSpawnerCommands),
{
    println!("here");
    spawner
        .spawn_entity((Node::default()))
        .with_children(|parent| {
            children(parent);
        });
}

trait Spawner<'a> {
    fn spawn_entity(&'a mut self, bundle: impl Bundle) -> EntityCommands<'a>;
}
struct Foo {}
impl<'a> Spawner<'a> for ChildSpawnerCommands<'_> {
    fn spawn_entity(&'a mut self, bundle: impl Bundle) -> EntityCommands<'a> {
        self.spawn(bundle)
    }
}

impl<'a> Spawner<'a> for Commands<'_, '_> {
    fn spawn_entity(&'a mut self, bundle: impl Bundle) -> EntityCommands<'a> {
        self.spawn(bundle)
    }
}
